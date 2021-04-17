use std::{sync::Arc, time::Instant};

use crate::{
    hit::Hit,
    math::{bounds::Bounds3, point::Point3, ray::Ray, vector::Vec3},
    shapes::Shape,
    yuki_info,
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies.html

#[derive(Copy, Clone)]
pub enum SplitMethod {
    Middle,
    EqualCounts,
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

        let mut ret = Self {
            split_method,
            max_shapes_in_node,
            nodes: Vec::new(),
            shapes: Arc::new(shapes),
        };

        let mut ordered_shapes = Vec::new();
        let build_start = Instant::now();
        let (root, node_count) =
            ret.recursive_build(&mut shape_info, 0, ret.shapes.len(), &mut ordered_shapes);
        yuki_info!(
            "BVH: Built the tree in {:.2}s",
            (build_start.elapsed().as_micros() as f32) * 1e-6
        );

        std::mem::swap(Arc::get_mut(&mut ret.shapes).unwrap(), &mut ordered_shapes);

        let flatten_start = Instant::now();
        ret.nodes = vec![BVHNode::default(); node_count];
        ret.flatten_tree(root, 0);
        yuki_info!(
            "BVH: Flattened the tree in {:.2}s",
            (flatten_start.elapsed().as_micros() as f32) * 1e-6
        );

        let shapes_arc = ret.shapes.clone();
        (ret, shapes_arc)
    }

    /// Intersects `ray` with the shapes in this `BoundingVolumeHierarchy`.
    pub fn intersect(&self, mut ray: Ray<f32>) -> Option<Hit> {
        let mut hit = None;

        // Pre-calculated to speed up Bounds3 intersection tests
        let inv_dir = Vec3::new(1.0 / ray.d.x, 1.0 / ray.d.y, 1.0 / ray.d.z);
        let dir_is_neg = [inv_dir.x < 0.0, inv_dir.y < 0.0, inv_dir.z < 0.0];

        let mut current_node_index = 0;
        // to_visit_index points to the next index to access in to_visit_stack
        let mut to_visit_index = 0;
        let mut to_visit_stack = [0; 64];
        loop {
            let node = self.nodes[current_node_index];
            if node.bounds.intersect(ray, inv_dir, dir_is_neg) {
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
                            current_node_index = current_node_index + 1;
                        }
                    }
                    NodeContent::Leaf {
                        first_shape_index,
                        shape_count,
                    } => {
                        let shape_range = (first_shape_index as usize)
                            ..((first_shape_index + (shape_count as u32)) as usize);
                        hit = self.shapes[shape_range].iter().fold(
                            hit.clone(),
                            |old_hit: Option<Hit>, shape| {
                                if let Some(new_hit) = shape.intersect(ray) {
                                    if let Some(old_hit) = old_hit {
                                        if new_hit.t < old_hit.t {
                                            ray.t_max = new_hit.t;
                                            Some(new_hit)
                                        } else {
                                            Some(old_hit)
                                        }
                                    } else {
                                        ray.t_max = new_hit.t;
                                        Some(new_hit)
                                    }
                                } else {
                                    old_hit
                                }
                            },
                        );

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
        hit
    }

    /// Builds the node structure as a [BVHBuildNode]-tree.
    ///
    /// Returns the root node and the total number of nodes in the tree.
    fn recursive_build(
        &mut self,
        shape_info: &mut Vec<BVHPrimitiveInfo>,
        start: usize,
        end: usize,
        ordered_shapes: &mut Vec<Arc<dyn Shape>>,
    ) -> (Box<BVHBuildNode>, usize) {
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
                (
                    BVHBuildNode::leaf(first_shape_index, shape_count, bounds),
                    1,
                )
            }};
        }

        if shape_count <= self.max_shapes_in_node {
            init_leaf!()
        } else {
            let centroid_bounds = shape_info[start..end]
                .iter()
                .fold(Bounds3::default(), |b, s| b.union_p(s.centroid));
            let axis = centroid_bounds.maximum_extent();

            if centroid_bounds.p_max[axis] == centroid_bounds.p_min[axis] {
                // No splitting method can help when bb is "zero"
                init_leaf!()
            } else {
                let mut mid = start;
                // We need to fall back to 'equal counts' if 'middle' fails
                let split_method = match self.split_method {
                    SplitMethod::Middle => {
                        // Partition shapes by their centroids on the two sides of the axis' middle point
                        let mid_value =
                            (centroid_bounds.p_min[axis] + centroid_bounds.p_max[axis]) / 2.0;
                        mid = itertools::partition(shape_info[start..end].iter_mut(), |s| {
                            s.centroid[axis] < mid_value
                        }) + start;

                        if mid != start && mid != end {
                            SplitMethod::Middle
                        } else {
                            SplitMethod::EqualCounts
                        }
                    }
                    _ => self.split_method,
                };

                match split_method {
                    SplitMethod::Middle => {}
                    SplitMethod::EqualCounts => {
                        // Partition shapes by their centroids into two sets with equal number of shapes
                        mid = (start + end) / 2;
                        shape_info[start..end].select_nth_unstable_by(mid - start, |a, b| {
                            a.centroid[axis]
                                .partial_cmp(&b.centroid[axis])
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }

                assert_ne!(mid, start, "BVH: Split failed");

                let (child0, child0_node_count) =
                    self.recursive_build(shape_info, start, mid, ordered_shapes);
                let (child1, child1_node_count) =
                    self.recursive_build(shape_info, mid, end, ordered_shapes);
                (
                    BVHBuildNode::interior(axis, child0, child1),
                    1 + child0_node_count + child1_node_count,
                )
            }
        }
    }

    /// Converts the [BVHBuildNode]-tree into a linear array of [BVHNode]s.
    ///
    /// Returns the next available index in the internal node array.
    fn flatten_tree(&mut self, root: Box<BVHBuildNode>, mut next_index: usize) -> usize {
        match root.content {
            BuildNodeContent::Interior {
                children: [child0, child1],
                split_axis,
            } => {
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

enum BuildNodeContent {
    Interior {
        children: [Box<BVHBuildNode>; 2],
        split_axis: usize,
    },
    /// Indexes into the ordered shape array.
    Leaf {
        first_shape_index: usize,
        shape_count: usize,
    },
}

struct BVHBuildNode {
    bounds: Bounds3<f32>,
    content: BuildNodeContent,
}

impl BVHBuildNode {
    /// Creates an interior `BVHBuildNode`.
    fn interior(
        split_axis: usize,
        child0: Box<BVHBuildNode>,
        child1: Box<BVHBuildNode>,
    ) -> Box<Self> {
        Box::new(Self {
            bounds: child0.bounds.union_b(child1.bounds),
            content: BuildNodeContent::Interior {
                children: [child0, child1],
                split_axis,
            },
        })
    }

    /// Creates a leaf `BVHBuildNode`.
    fn leaf(first_shape_index: usize, shape_count: usize, bounds: Bounds3<f32>) -> Box<Self> {
        Box::new(Self {
            bounds,
            content: BuildNodeContent::Leaf {
                first_shape_index,
                shape_count,
            },
        })
    }
}
