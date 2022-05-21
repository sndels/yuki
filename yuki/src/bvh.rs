use allocators::{LinearAllocator, ScopedScratch};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc, time::Instant};
use strum::{Display, EnumString, EnumVariantNames};

use crate::{
    lights::AreaLight,
    math::{Bounds3, Point3, Ray, Vec3},
    shapes::{Hit, Shape},
    yuki_info,
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies.html

#[derive(Copy, Clone, Deserialize, Serialize, Display, EnumVariantNames, EnumString)]
pub enum SplitMethod {
    SurfaceAreaHeuristic,
    Middle,
    EqualCounts,
}

pub struct IntersectionResult<'a> {
    pub hit: Option<Hit<'a>>,
    pub intersection_test_count: usize,
    pub intersection_count: usize,
}

/// A standard BVH.
pub struct BoundingVolumeHierarchy {
    split_method: SplitMethod,
    max_shapes_in_node: usize,
    nodes: Vec<BVHNode>,
    shapes: Arc<Vec<Arc<dyn Shape>>>,
}

impl BoundingVolumeHierarchy {
    /// Creates a new `BoundingVolumeHierarchy` for the given [Shape]s. Also returns back `shapes` as an Arc.
    pub fn new(
        shapes: Vec<Arc<dyn Shape>>,
        max_shapes_in_node: usize,
        split_method: SplitMethod,
    ) -> (Self, Arc<Vec<Arc<dyn Shape>>>) {
        superluminal_perf::begin_event("bvh build");

        superluminal_perf::begin_event("bounds setup");

        let mut bounds = Bounds3::default();
        let mut shape_info = Vec::new();
        for (i, s) in shapes.iter().enumerate() {
            let b = s.world_bound();
            bounds = bounds.union_b(b);
            shape_info.push(BVHPrimitiveInfo {
                shape_index: i,
                bounds: b,
                centroid: b.p_min + (b.diagonal() / 0.5),
            });
        }

        superluminal_perf::end_event(); // bounds setup

        let mut ret = Self {
            split_method,
            max_shapes_in_node,
            nodes: Vec::new(),
            shapes: Arc::new(shapes),
        };

        let mut ordered_shapes = Vec::new();
        let build_start = Instant::now();

        superluminal_perf::begin_event("recursive build");

        let mut allocator =
            LinearAllocator::new(std::mem::size_of::<BVHBuildNode>() * (ret.shapes.len() * 2 - 1));
        let scratch = ScopedScratch::new(&mut allocator);

        let RecursiveBuildResult {
            root,
            nodes_in_tree,
        } = ret.recursive_build(
            &scratch,
            &mut shape_info,
            0,
            ret.shapes.len(),
            &mut ordered_shapes,
        );

        superluminal_perf::end_event(); // recursive build

        yuki_info!(
            "BVH: Built the tree in {:.2}s",
            build_start.elapsed().as_secs_f32()
        );

        std::mem::swap(Arc::get_mut(&mut ret.shapes).unwrap(), &mut ordered_shapes);

        let flatten_start = Instant::now();
        superluminal_perf::begin_event("tree flattening");

        ret.nodes = vec![BVHNode::default(); nodes_in_tree];
        ret.flatten_tree(root, 0);

        superluminal_perf::end_event(); // tree flattening

        yuki_info!(
            "BVH: Flattened the tree in {:.2}s",
            flatten_start.elapsed().as_secs_f32()
        );

        superluminal_perf::end_event(); // bvh build

        let shapes_arc = Arc::clone(&ret.shapes);
        (ret, shapes_arc)
    }

    pub fn bounds(&self) -> Bounds3<f32> {
        self.nodes[0].bounds
    }

    pub fn node_bounds(&self, target_level: i32) -> Vec<Bounds3<f32>> {
        struct Node {
            index: usize,
            level: i32,
        }

        let mut bounds = vec![];
        if target_level <= 0 {
            bounds.push(self.nodes[0].bounds);
        }
        let mut stack = VecDeque::from([Node { index: 0, level: 1 }]);
        while !stack.is_empty() {
            let Node { index, level } = stack.pop_front().unwrap();
            if target_level >= 0 && level > target_level {
                break;
            }

            if let NodeContent::Interior {
                second_child_index, ..
            } = self.nodes[index].content
            {
                if target_level < 0 || level == target_level {
                    bounds.push(self.nodes[index + 1].bounds);
                    bounds.push(self.nodes[second_child_index as usize].bounds);
                }
                stack.push_back(Node {
                    index: index + 1,
                    level: level + 1,
                });
                stack.push_back(Node {
                    index: second_child_index as usize,
                    level: level + 1,
                });
            }
        }
        bounds
    }

    /// Intersects `ray` with the shapes in this `BoundingVolumeHierarchy`.
    pub fn intersect<'a>(
        &'a self,
        scratch: &'a ScopedScratch,
        mut ray: Ray<f32>,
    ) -> IntersectionResult<'a> {
        let mut hit: Option<Hit> = None;

        // Pre-calculated to speed up Bounds3 intersection tests
        let inv_dir = Vec3::new(1.0 / ray.d.x, 1.0 / ray.d.y, 1.0 / ray.d.z);
        let dir_is_neg = [inv_dir.x < 0.0, inv_dir.y < 0.0, inv_dir.z < 0.0];

        let mut intersection_test_count = 0;
        let mut intersection_count = 0;
        let mut current_node_index = 0;
        // to_visit_index points to the next index to access in to_visit_stack
        let mut to_visit_index = 0;
        let mut to_visit_stack = [0; 64];
        loop {
            assert!(to_visit_index < to_visit_stack.len());

            let node = &self.nodes[current_node_index];
            intersection_test_count += 1;
            if node.bounds.intersect(ray, inv_dir) {
                intersection_count += 1;
                match node.content {
                    NodeContent::Interior {
                        second_child_index,
                        split_axis,
                    } => {
                        // Traverse children front to back
                        if dir_is_neg[split_axis as usize] {
                            to_visit_stack[to_visit_index] = current_node_index + 1;
                            to_visit_index += 1;
                            current_node_index = second_child_index as usize;
                        } else {
                            to_visit_stack[to_visit_index] = second_child_index as usize;
                            to_visit_index += 1;
                            current_node_index += 1;
                        }
                    }
                    NodeContent::Leaf {
                        first_shape_index,
                        shape_count,
                    } => {
                        let shape_range = (first_shape_index as usize)
                            ..((first_shape_index + (shape_count as u32)) as usize);
                        for shape in &self.shapes[shape_range] {
                            if let Some(mut new_hit) = shape.intersect(ray) {
                                if let Some(old_hit) = hit.as_ref() {
                                    if new_hit.t < old_hit.t {
                                        ray.t_max = new_hit.t;
                                        new_hit.bsdf = Some(
                                            shape
                                                .compute_scattering_functions(scratch, &new_hit.si),
                                        );
                                        hit = Some(new_hit);
                                    }
                                } else {
                                    ray.t_max = new_hit.t;
                                    new_hit.bsdf = Some(
                                        shape.compute_scattering_functions(scratch, &new_hit.si),
                                    );
                                    hit = Some(new_hit);
                                }
                            }
                        }

                        if to_visit_index == 0 {
                            break;
                        }

                        to_visit_index -= 1;
                        current_node_index = to_visit_stack[to_visit_index];
                    }
                    NodeContent::Uninitialized => unreachable!(),
                }
            } else {
                if to_visit_index == 0 {
                    break;
                }
                to_visit_index -= 1;
                current_node_index = to_visit_stack[to_visit_index];
            }
        }
        IntersectionResult {
            hit,
            intersection_test_count,
            intersection_count,
        }
    }

    /// Checks if`ray` intersects with any of the shapes in this `BoundingVolumeHierarchy`.
    pub fn any_intersect(&self, ray: Ray<f32>, area_light: Option<&dyn AreaLight>) -> bool {
        // Pre-calculated to speed up Bounds3 intersection tests
        let inv_dir = Vec3::new(1.0 / ray.d.x, 1.0 / ray.d.y, 1.0 / ray.d.z);

        let mut current_node_index = 0;
        // to_visit_index points to the next index to access in to_visit_stack
        let mut to_visit_index = 0;
        let mut to_visit_stack = [0; 64];
        loop {
            let node = &self.nodes[current_node_index];
            if node.bounds.intersect(ray, inv_dir) {
                match node.content {
                    NodeContent::Interior {
                        second_child_index,
                        split_axis,
                    } => {
                        // Traverse children front to back
                        if inv_dir[split_axis as usize] < 0.0 {
                            to_visit_stack[to_visit_index] = current_node_index + 1;
                            to_visit_index += 1;
                            current_node_index = second_child_index as usize;
                        } else {
                            to_visit_stack[to_visit_index] = second_child_index as usize;
                            to_visit_index += 1;
                            current_node_index += 1;
                        }
                    }
                    NodeContent::Leaf {
                        first_shape_index,
                        shape_count,
                    } => {
                        let shape_range = (first_shape_index as usize)
                            ..((first_shape_index + (shape_count as u32)) as usize);
                        for shape in &self.shapes[shape_range] {
                            if let Some(Hit { si, .. }) = shape.intersect(ray) {
                                if let (Some(target_l), Some(l)) = (area_light, si.area_light) {
                                    if !std::ptr::eq(
                                        (l.as_ref() as *const dyn AreaLight).cast::<()>(),
                                        (target_l as *const dyn AreaLight).cast::<()>(),
                                    ) {
                                        return true;
                                    }
                                } else {
                                    return true;
                                }
                            }
                        }

                        if to_visit_index == 0 {
                            break;
                        }

                        to_visit_index -= 1;
                        current_node_index = to_visit_stack[to_visit_index];
                    }
                    NodeContent::Uninitialized => unreachable!(),
                }
            } else {
                if to_visit_index == 0 {
                    break;
                }
                to_visit_index -= 1;
                current_node_index = to_visit_stack[to_visit_index];
            }
        }

        false
    }

    /// Builds the node structure as a [BVHBuildNode]-tree.
    fn recursive_build<'a>(
        &mut self,
        scratch: &'a ScopedScratch,
        shape_info: &mut [BVHPrimitiveInfo],
        start: usize,
        end: usize,
        ordered_shapes: &mut Vec<Arc<dyn Shape>>,
    ) -> RecursiveBuildResult<'a> {
        let bounds = shape_info[start..end]
            .iter()
            .fold(Bounds3::default(), |b, s| b.union_b(s.bounds));
        let first_shape_index = ordered_shapes.len();

        let shape_count = end - start;

        macro_rules! init_leaf {
            () => {{
                ordered_shapes.extend(
                    shape_info[start..end]
                        .iter()
                        .map(|s| self.shapes[s.shape_index].clone()),
                );
                RecursiveBuildResult {
                    root: scratch.alloc(BVHBuildNode::leaf(first_shape_index, shape_count, bounds)),
                    nodes_in_tree: 1,
                }
            }};
        }

        if shape_count <= self.max_shapes_in_node {
            init_leaf!()
        } else {
            let centroid_bounds = shape_info[start..end]
                .iter()
                .fold(Bounds3::default(), |b, s| b.union_p(s.centroid));
            let axis = centroid_bounds.maximum_extent();

            #[allow(clippy::float_cmp)] // We really do want the exact case
            if centroid_bounds.p_max[axis] == centroid_bounds.p_min[axis] {
                // No splitting method can help when bb is "zero"
                init_leaf!()
            } else {
                let mid = match self.split_method {
                    SplitMethod::SurfaceAreaHeuristic => {
                        let mid =
                            split_sah(shape_info, &bounds, &centroid_bounds, start, end, axis);
                        if mid != start && mid != end {
                            mid
                        } else {
                            split_equal_counts(shape_info, start, end, axis)
                        }
                    }
                    SplitMethod::Middle => {
                        let mid = split_middle(shape_info, &centroid_bounds, start, end, axis);
                        if mid != start && mid != end {
                            mid
                        } else {
                            split_equal_counts(shape_info, start, end, axis)
                        }
                    }
                    SplitMethod::EqualCounts => split_equal_counts(shape_info, start, end, axis),
                };

                assert_ne!(mid, start, "BVH: Split failed");

                // TODO: Just use enum Split(mid)/Leaf/Failed here?
                if mid == usize::MAX {
                    init_leaf!()
                } else {
                    let RecursiveBuildResult {
                        root: child0,
                        nodes_in_tree: child0_node_count,
                    } = self.recursive_build(scratch, shape_info, start, mid, ordered_shapes);
                    let RecursiveBuildResult {
                        root: child1,
                        nodes_in_tree: child1_node_count,
                    } = self.recursive_build(scratch, shape_info, mid, end, ordered_shapes);

                    RecursiveBuildResult {
                        root: scratch.alloc(BVHBuildNode::interior(axis, child0, child1)),
                        nodes_in_tree: 1 + child0_node_count + child1_node_count,
                    }
                }
            }
        }
    }

    /// Converts the [BVHBuildNode]-tree into a linear array of [BVHNode]s.
    ///
    /// Returns the next available index in the internal node array.
    #[allow(clippy::boxed_local)] // Box is more convenient here as the input is boxed anyway
    fn flatten_tree(&mut self, root: &BVHBuildNode, mut next_index: usize) -> usize {
        match root.content {
            BuildNodeContent::Interior {
                child0,
                child1,
                split_axis,
            } => {
                // TODO: Flatten with the two children together?
                let self_index = next_index;
                let second_child_index = self.flatten_tree(child0, self_index + 1);
                next_index = self.flatten_tree(child1, second_child_index);
                self.nodes[self_index] =
                    BVHNode::interior(root.bounds, second_child_index, split_axis);
            }
            BuildNodeContent::Leaf {
                first_shape_index,
                shape_count,
            } => {
                self.nodes[next_index] = BVHNode::leaf(root.bounds, first_shape_index, shape_count);
                next_index += 1;
            }
        }
        next_index
    }
}

fn split_equal_counts(
    shape_info: &mut [BVHPrimitiveInfo],
    start: usize,
    end: usize,
    axis: usize,
) -> usize {
    // Partition shapes by their centroids into two sets with equal number of shapes
    let mid = (start + end) / 2;
    shape_info[start..end].select_nth_unstable_by(mid - start, |a, b| {
        a.centroid[axis]
            .partial_cmp(&b.centroid[axis])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    mid
}

fn split_middle(
    shape_info: &mut [BVHPrimitiveInfo],
    centroid_bounds: &Bounds3<f32>,
    start: usize,
    end: usize,
    axis: usize,
) -> usize {
    // Partition shapes by their centroids on the two sides of the axis' middle point
    let mid_value = (centroid_bounds.p_min[axis] + centroid_bounds.p_max[axis]) / 2.0;
    itertools::partition(shape_info[start..end].iter_mut(), |s| {
        s.centroid[axis] < mid_value
    }) + start
}

fn split_sah(
    shape_info: &mut [BVHPrimitiveInfo],
    bounds: &Bounds3<f32>,
    centroid_bounds: &Bounds3<f32>,
    start: usize,
    end: usize,
    axis: usize,
) -> usize {
    let shape_count = end - start;
    if shape_count <= 2 {
        start
    } else {
        const N_BUCKETS: usize = 12;
        #[derive(Clone, Copy)]
        struct BucketInfo {
            count: usize,
            bounds: Bounds3<f32>,
        }

        // Sort shapes into N buckets
        let mut buckets = [BucketInfo {
            count: 0,
            bounds: Bounds3::default(),
        }; N_BUCKETS];
        for si in &shape_info[start..end] {
            let bf = N_BUCKETS as f32 * centroid_bounds.offset(si.centroid)[axis];
            #[allow(clippy::cast_sign_loss)] // Explicit max is used
            let b = (bf.max(0.0) as usize).min(N_BUCKETS - 1);

            buckets[b].count += 1;
            buckets[b].bounds = buckets[b].bounds.union_b(si.bounds);
        }

        // Evaluate
        let mut costs = [0.0f32; N_BUCKETS - 1];
        for (i, cost) in costs.iter_mut().enumerate() {
            let (b0, count0) = buckets[0..=i]
                .iter()
                .fold((Bounds3::<f32>::default(), 0), |(b, c), bucket| {
                    (b.union_b(bucket.bounds), c + bucket.count)
                });
            let (b1, count1) = buckets[(i + 1)..]
                .iter()
                .fold((Bounds3::<f32>::default(), 0), |(b, c), bucket| {
                    (b.union_b(bucket.bounds), c + bucket.count)
                });
            *cost = 1.0
                + ((count0 as f32) * b0.surface_area() + (count1 as f32) * b1.surface_area())
                    / bounds.surface_area().max(1e-10);
        }

        // Pick best
        let (min_cost_split_bucket, &min_cost) = costs
            .iter()
            .enumerate()
            .min_by(|(_, c0), (_, c1)| c0.partial_cmp(c1).unwrap())
            .unwrap();

        let leaf_cost = shape_count as f32;
        if min_cost < leaf_cost {
            itertools::partition(shape_info[start..end].iter_mut(), |s| {
                let bf = N_BUCKETS as f32 * centroid_bounds.offset(s.centroid)[axis];
                #[allow(clippy::cast_sign_loss)] // Explicit max is used
                let b = (bf.max(0.0) as usize).min(N_BUCKETS - 1);

                b <= min_cost_split_bucket
            }) + start
        } else {
            usize::MAX
        }
    }
}

struct RecursiveBuildResult<'a> {
    root: &'a BVHBuildNode<'a>,
    nodes_in_tree: usize,
}

struct BVHPrimitiveInfo {
    shape_index: usize,
    bounds: Bounds3<f32>,
    centroid: Point3<f32>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum NodeContent {
    /// Indexes into the node array.
    Interior {
        second_child_index: u32,
        split_axis: u8,
    },
    /// Indexes into the ordered shape array.
    Leaf {
        first_shape_index: u32,
        shape_count: u16,
    },
    Uninitialized,
}

#[derive(Copy, Clone)]
struct BVHNode {
    bounds: Bounds3<f32>,
    content: NodeContent,
}

impl BVHNode {
    /// Creates an uninitialized `BVHNode`.
    fn default() -> Self {
        Self {
            bounds: Bounds3::default(),
            content: NodeContent::Uninitialized,
        }
    }

    /// Creates an interior `BVHNode`.
    fn interior(bounds: Bounds3<f32>, second_child_index: usize, split_axis: usize) -> Self {
        Self {
            bounds,
            content: NodeContent::Interior {
                second_child_index: second_child_index as u32,
                split_axis: split_axis as u8,
            },
        }
    }

    /// Creates a leaf `BVHNode`.
    fn leaf(bounds: Bounds3<f32>, first_shape_index: usize, shape_count: usize) -> Self {
        Self {
            bounds,
            content: NodeContent::Leaf {
                first_shape_index: first_shape_index as u32,
                shape_count: shape_count as u16,
            },
        }
    }
}

enum BuildNodeContent<'a> {
    Interior {
        child0: &'a BVHBuildNode<'a>,
        child1: &'a BVHBuildNode<'a>,
        split_axis: usize,
    },
    /// Indexes into the ordered shape array.
    Leaf {
        first_shape_index: usize,
        shape_count: usize,
    },
}

struct BVHBuildNode<'a> {
    bounds: Bounds3<f32>,
    content: BuildNodeContent<'a>,
}

impl<'a> BVHBuildNode<'a> {
    /// Creates an interior `BVHBuildNode`.
    fn interior(split_axis: usize, child0: &'a BVHBuildNode, child1: &'a BVHBuildNode) -> Self {
        Self {
            bounds: child0.bounds.union_b(child1.bounds),
            content: BuildNodeContent::Interior {
                child0,
                child1,
                split_axis,
            },
        }
    }

    /// Creates a leaf `BVHBuildNode`.
    fn leaf(first_shape_index: usize, shape_count: usize, bounds: Bounds3<f32>) -> Self {
        Self {
            bounds,
            content: BuildNodeContent::Leaf {
                first_shape_index,
                shape_count,
            },
        }
    }
}
