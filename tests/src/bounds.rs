#[cfg(test)]
mod tests {
    use num::Bounded;

    use yuki::bounds::{Bounds2, Bounds3};
    use yuki::point::{point2, point3, Point2};
    use yuki::vector::{vec2, vec3, Vec2};

    #[test]
    fn new() {
        let bb = Bounds2::new(point2(0, 0), point2(1, 1));
        assert_eq!(bb.p_min, point2(0, 0));
        assert_eq!(bb.p_max, point2(1, 1));
        let bb = Bounds2::new(point2(1, 1), point2(0, 0));
        assert_eq!(bb.p_min, point2(0, 0));
        assert_eq!(bb.p_max, point2(1, 1));

        let bb = Bounds3::new(point3(0, 0, 0), point3(1, 1, 1));
        assert_eq!(bb.p_min, point3(0, 0, 0));
        assert_eq!(bb.p_max, point3(1, 1, 1));
        let bb = Bounds3::new(point3(1, 1, 1), point3(0, 0, 0));
        assert_eq!(bb.p_min, point3(0, 0, 0));
        assert_eq!(bb.p_max, point3(1, 1, 1));
    }

    #[test]
    fn default() {
        let bb = Bounds2::<f32>::default();
        for i in 0..2 {
            assert_eq!(bb.p_max[i], f32::min_value());
            assert_eq!(bb.p_min[i], f32::max_value());
        }
        let bb = Bounds3::<f32>::default();
        for i in 0..3 {
            assert_eq!(bb.p_max[i], f32::min_value());
            assert_eq!(bb.p_min[i], f32::max_value());
        }
    }

    #[test]
    fn index() {
        let bb = Bounds2::new(point2(0, 0), point2(1, 1));
        assert_eq!(bb[0], point2(0, 0));
        assert_eq!(bb[1], point2(1, 1));

        let bb = Bounds3::new(point3(0, 0, 0), point3(1, 1, 1));
        assert_eq!(bb[0], point3(0, 0, 0));
        assert_eq!(bb[1], point3(1, 1, 1));
    }

    #[test]
    fn index_mut() {
        let mut bb = Bounds2::new(point2(0, 0), point2(1, 1));
        bb[0][0] = 2;
        bb[1][0] = 3;
        assert_eq!(bb[0], point2(2, 0));
        assert_eq!(bb[1], point2(3, 1));

        let mut bb = Bounds3::new(point3(0, 0, 0), point3(1, 1, 1));
        bb[0][0] = 2;
        bb[1][0] = 3;
        assert_eq!(bb[0], point3(2, 0, 0));
        assert_eq!(bb[1], point3(3, 1, 1));
    }

    #[test]
    fn union_p() {
        let bb = Bounds2::new(point2(0, 0), point2(2, 2));
        assert_eq!(bb.union_p(point2(1, 1)), bb);
        assert_eq!(
            bb.union_p(point2(3, 1)),
            Bounds2::new(point2(0, 0), point2(3, 2))
        );
        assert_eq!(
            bb.union_p(point2(3, 4)),
            Bounds2::new(point2(0, 0), point2(3, 4))
        );
        assert_eq!(
            bb.union_p(point2(-3, -4)),
            Bounds2::new(point2(-3, -4), point2(2, 2))
        );

        let bb = Bounds3::new(point3(0, 0, 0), point3(2, 2, 2));
        assert_eq!(bb.union_p(point3(1, 1, 1)), bb);
    }

    #[test]
    fn union_b() {
        let points = [point2(0, 0), point2(1, 1), point2(2, 2), point2(3, 3)];
        let bbu = Bounds2::new(points[0], points[3]);
        for l in 0..4 {
            for k in 0..4 {
                if k != l {
                    for j in 0..4 {
                        if j != l && j != k {
                            for i in 0..4 {
                                if i != l && i != k && i != j {
                                    let bb0 = Bounds2::new(points[l], points[k]);
                                    let bb1 = Bounds2::new(points[j], points[i]);
                                    assert_eq!(bb0.union_b(bb1), bbu);
                                }
                            }
                        }
                    }
                }
            }
        }
        let bbi = Bounds2::new(points[1], points[2]);
        assert_eq!(bbi.union_b(bbu), bbu);
        assert_eq!(bbu.union_b(bbi), bbu);

        let bb0 = Bounds3::new(point3(0, 0, 0), point3(2, 2, 2));
        let bb1 = Bounds3::new(point3(1, 1, 1), point3(1, 1, 1));
        assert_eq!(bb0.union_b(bb1), bb0);
        assert_eq!(bb1.union_b(bb0), bb0);
    }

    #[test]
    fn intersection() {
        let bb0 = Bounds2::new(point2(0, 1), point2(2, 3));
        for j in -1..=1 {
            for i in -1..=1 {
                let offset = point2(i, j);
                let bb1 = Bounds2::new(point2(0, 1) + offset, point2(2, 3) + offset);
                let (xmin, xmax) = if i < 0 {
                    (0, 1)
                } else if i == 0 {
                    (0, 2)
                } else {
                    (1, 2)
                };
                let (ymin, ymax) = if j < 0 {
                    (1, 2)
                } else if j == 0 {
                    (1, 3)
                } else {
                    (2, 3)
                };
                let bbi = Bounds2::new(point2(xmin, ymin), point2(xmax, ymax));
                assert_eq!(bb0.intersection(bb1).unwrap(), bbi);
            }
        }
        let bb0 = Bounds2::new(point2(0, 0), point2(1, 1));
        let bb1 = Bounds2::new(point2(-1, -1), point2(2, 2));
        assert_eq!(bb0.intersection(bb1).unwrap(), bb0);
        assert_eq!(bb1.intersection(bb0).unwrap(), bb0);
        let bb0 = Bounds2::new(point2(0, 0), point2(1, 1));
        let bb1 = Bounds2::new(point2(2, 2), point2(3, 3));
        assert!(bb0.intersection(bb1).is_none());

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(point3(0, 1, 2), point3(3, 4, 5));
        let bb1 = Bounds3::new(point3(-1, -2, -3), point3(1, 2, 3));
        assert_eq!(
            bb0.intersection(bb1).unwrap(),
            Bounds3::new(point3(0, 1, 2), point3(1, 2, 3))
        );
    }

    #[test]
    fn expanded() {
        let bb0 = Bounds2::new(point2(0, 1), point2(2, 3));
        assert_eq!(
            bb0.expanded(1).unwrap(),
            Bounds2::new(point2(-1, 0), point2(3, 4))
        );
        assert_eq!(
            bb0.expanded(-1).unwrap(),
            Bounds2::new(point2(1, 2), point2(1, 2))
        );
        assert!(Bounds2::new(point2(0, 0), point2(1, 4))
            .expanded(-2)
            .is_none());
        assert!(Bounds2::new(point2(0, 0), point2(4, 1))
            .expanded(-2)
            .is_none());

        let bb0 = Bounds3::new(point3(0, 1, 2), point3(3, 4, 5));
        assert_eq!(
            bb0.expanded(1).unwrap(),
            Bounds3::new(point3(-1, 0, 1), point3(4, 5, 6))
        );
        assert_eq!(
            bb0.expanded(-1).unwrap(),
            Bounds3::new(point3(1, 2, 3), point3(2, 3, 4))
        );
    }

    #[test]
    fn overlaps() {
        let bb0 = Bounds2::new(point2(0, 1), point2(2, 3));
        for j in -1..=1 {
            for i in -1..=1 {
                let offset = point2(i, j);
                let bb1 = Bounds2::new(point2(0, 1) + offset, point2(2, 3) + offset);
                assert!(bb0.overlaps(bb1));
            }
        }
        let bb0 = Bounds2::new(point2(0, 0), point2(1, 1));
        let bb1 = Bounds2::new(point2(-1, -1), point2(2, 2));
        assert!(bb0.overlaps(bb1));
        assert!(bb1.overlaps(bb0));
        let bb0 = Bounds2::new(point2(0, 0), point2(1, 1));
        let bb1 = Bounds2::new(point2(2, 2), point2(3, 3));
        assert!(!bb0.overlaps(bb1));

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(point3(0, 1, 2), point3(3, 4, 5));
        let bb1 = Bounds3::new(point3(-1, -2, -3), point3(1, 2, 3));
        assert!(bb0.overlaps(bb1));
    }

    #[test]
    fn inside() {
        let bb0 = Bounds2::new(point2(0, 2), point2(3, 5));
        for j in 2..=5 {
            for i in 0..=3 {
                assert!(bb0.inside(point2(i, j)));
            }
        }
        for &(i, j) in &[
            (-1, 1),
            (1, 1),
            (4, 1),
            (-1, 4),
            (4, 4),
            (-1, 6),
            (1, 6),
            (4, 6),
        ] {
            assert!(!bb0.inside(point2(i, j)));
        }

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(point3(0, 1, 2), point3(3, 4, 5));
        assert!(bb0.inside(point3(1, 2, 3)));
    }

    #[test]
    fn inside_exclusive() {
        let bb0 = Bounds2::new(point2(0, 2), point2(3, 5));
        for j in 2..5 {
            for i in 0..3 {
                assert!(bb0.inside_exclusive(point2(i, j)));
            }
        }
        for &(i, j) in &[
            (-1, 1),
            (1, 1),
            (4, 1),
            (-1, 4),
            (3, 4),
            (4, 4),
            (1, 5),
            (3, 5),
            (-1, 6),
            (1, 6),
            (4, 6),
        ] {
            assert!(!bb0.inside_exclusive(point2(i, j)));
        }

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(point3(0, 1, 2), point3(3, 4, 5));
        assert!(bb0.inside_exclusive(point3(1, 2, 3)));
    }

    #[test]
    fn diagonal() {
        let bb = Bounds2::new(point2(1, 2), point2(4, 7));
        assert_eq!(bb.diagonal(), vec2(3, 5));
        let bb = Bounds3::new(point3(1, 2, 3), point3(5, 8, 11));
        assert_eq!(bb.diagonal(), vec3(4, 6, 8));
    }

    #[test]
    fn lerp() {
        let p0 = point2(1, 2);
        let p1 = point2(3, 4);
        let bb = Bounds2::new(p0, p1);
        for i in 0..2 {
            let mut t0 = Point2::zeros();
            t0[i] = 0.5;
            let mut pt = p0;
            pt[i] += 1;
            assert_eq!(bb.lerp(t0), pt);

            let mut t0 = Point2::zeros();
            t0[i] = 2.0;
            let mut pt = p0;
            pt[i] += 4;
            assert_eq!(bb.lerp(t0), pt);

            let mut t0 = Point2::zeros();
            t0[i] = -1.0;
            let mut pt = p0;
            pt[i] -= 2;
            assert_eq!(bb.lerp(t0), pt);
        }

        let bb = Bounds3::new(point3(1, 2, 3), point3(4, 5, 6));
        // Tests both existence of the func and rounding
        assert_eq!(bb.lerp(point3(0.5, 0.5, 0.5)), point3(2, 3, 4));
    }

    #[test]
    fn offset() {
        let p0 = point2(1.0, 2.0);
        let p1 = point2(3.0, 4.0);
        let bb = Bounds2::new(p0, p1);
        for i in 0..2 {
            let mut pp = p0;
            let mut pd = Vec2::zeros();
            assert_eq!(bb.offset(pp), pd);

            pp[i] += 1.0;
            pd[i] = 0.5;
            assert_eq!(bb.offset(pp), pd);

            pp[i] += 1.0;
            pd[i] = 1.0;
            assert_eq!(bb.offset(pp), pd);

            pp[i] += 2.0;
            pd[i] = 2.0;
            assert_eq!(bb.offset(pp), pd);

            pp[i] -= 6.0;
            pd[i] = -1.0;
            assert_eq!(bb.offset(pp), pd);
        }

        let bb = Bounds3::new(point3(1.0, 2.0, 3.0), point3(4.0, 5.0, 6.0));
        // Tests both existence of the func and rounding
        assert_eq!(bb.offset(point3(2.5, 3.5, 4.5)), vec3(0.5, 0.5, 0.5));
    }

    #[test]
    fn area() {
        let bb = Bounds2::new(point2(1, 2), point2(3, 5));
        assert_eq!(bb.area(), 6);
        let bb = Bounds2::new(point2(-1, -2), point2(-3, -5));
        assert_eq!(bb.area(), 6);
    }

    #[test]
    fn surface_area() {
        let bb = Bounds3::new(point3(1, 2, 3), point3(3, 5, 7));
        assert_eq!(bb.surface_area(), 52);
        let bb = Bounds3::new(point3(-1, -2, -3), point3(-3, -5, -7));
        assert_eq!(bb.surface_area(), 52);
    }

    #[test]
    fn volume() {
        let bb = Bounds3::new(point3(1, 2, 3), point3(3, 5, 7));
        assert_eq!(bb.volume(), 24);
        let bb = Bounds3::new(point3(-1, -2, -3), point3(-3, -5, -7));
        assert_eq!(bb.volume(), 24);
    }

    #[test]
    fn maximum_extent() {
        let bb = Bounds2::new(point2(1, 2), point2(3, 5));
        assert_eq!(bb.maximum_extent(), 1);
        let bb = Bounds2::new(point2(1, 2), point2(4, 3));
        assert_eq!(bb.maximum_extent(), 0);

        let bb = Bounds3::new(point3(1, 2, 3), point3(4, 5, 7));
        assert_eq!(bb.maximum_extent(), 2);
        let bb = Bounds3::new(point3(1, 2, 3), point3(4, 6, 5));
        assert_eq!(bb.maximum_extent(), 1);
        let bb = Bounds3::new(point3(1, 2, 3), point3(7, 5, 6));
        assert_eq!(bb.maximum_extent(), 0);
    }

    #[test]
    fn bounding_sphere() {
        let p0 = point3(1.0, 2.0, 3.0);
        let p1 = point3(4.0, 5.0, 6.0);
        let center = point3(2.5, 3.5, 4.5);
        let radius = center.dist(p1);
        let bb = Bounds3::new(p0, p1);
        assert_eq!(bb.bounding_sphere().unwrap(), (center, radius));

        let bb = Bounds3 {
            p_min: point3(5.0, 6.0, 7.0),
            p_max: p1,
        };
        assert!(bb.bounding_sphere().is_none());
    }
}
