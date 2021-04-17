#[cfg(test)]
mod tests {
    use num::Bounded;

    use yuki::math::{Bounds2, Bounds3, Point2, Point3, Vec2, Vec3};

    #[test]
    fn new() {
        let bb = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        assert_eq!(bb.p_min, Point2::new(0, 0));
        assert_eq!(bb.p_max, Point2::new(1, 1));
        let bb = Bounds2::new(Point2::new(1, 1), Point2::new(0, 0));
        assert_eq!(bb.p_min, Point2::new(0, 0));
        assert_eq!(bb.p_max, Point2::new(1, 1));

        let bb = Bounds3::new(Point3::new(0, 0, 0), Point3::new(1, 1, 1));
        assert_eq!(bb.p_min, Point3::new(0, 0, 0));
        assert_eq!(bb.p_max, Point3::new(1, 1, 1));
        let bb = Bounds3::new(Point3::new(1, 1, 1), Point3::new(0, 0, 0));
        assert_eq!(bb.p_min, Point3::new(0, 0, 0));
        assert_eq!(bb.p_max, Point3::new(1, 1, 1));
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
        let bb = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        assert_eq!(bb[0], Point2::new(0, 0));
        assert_eq!(bb[1], Point2::new(1, 1));

        let bb = Bounds3::new(Point3::new(0, 0, 0), Point3::new(1, 1, 1));
        assert_eq!(bb[0], Point3::new(0, 0, 0));
        assert_eq!(bb[1], Point3::new(1, 1, 1));
    }

    #[test]
    fn index_mut() {
        let mut bb = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        bb[0][0] = 2;
        bb[1][0] = 3;
        assert_eq!(bb[0], Point2::new(2, 0));
        assert_eq!(bb[1], Point2::new(3, 1));

        let mut bb = Bounds3::new(Point3::new(0, 0, 0), Point3::new(1, 1, 1));
        bb[0][0] = 2;
        bb[1][0] = 3;
        assert_eq!(bb[0], Point3::new(2, 0, 0));
        assert_eq!(bb[1], Point3::new(3, 1, 1));
    }

    #[test]
    fn union_p() {
        let bb = Bounds2::new(Point2::new(0, 0), Point2::new(2, 2));
        assert_eq!(bb.union_p(Point2::new(1, 1)), bb);
        assert_eq!(
            bb.union_p(Point2::new(3, 1)),
            Bounds2::new(Point2::new(0, 0), Point2::new(3, 2))
        );
        assert_eq!(
            bb.union_p(Point2::new(3, 4)),
            Bounds2::new(Point2::new(0, 0), Point2::new(3, 4))
        );
        assert_eq!(
            bb.union_p(Point2::new(-3, -4)),
            Bounds2::new(Point2::new(-3, -4), Point2::new(2, 2))
        );

        let bb = Bounds3::new(Point3::new(0, 0, 0), Point3::new(2, 2, 2));
        assert_eq!(bb.union_p(Point3::new(1, 1, 1)), bb);
    }

    #[test]
    fn union_b() {
        let points = [
            Point2::new(0, 0),
            Point2::new(1, 1),
            Point2::new(2, 2),
            Point2::new(3, 3),
        ];
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

        let bb0 = Bounds3::new(Point3::new(0, 0, 0), Point3::new(2, 2, 2));
        let bb1 = Bounds3::new(Point3::new(1, 1, 1), Point3::new(1, 1, 1));
        assert_eq!(bb0.union_b(bb1), bb0);
        assert_eq!(bb1.union_b(bb0), bb0);
    }

    #[test]
    fn intersection() {
        let bb0 = Bounds2::new(Point2::new(0, 1), Point2::new(2, 3));
        for j in -1..=1 {
            for i in -1..=1 {
                let offset = Point2::new(i, j);
                let bb1 = Bounds2::new(Point2::new(0, 1) + offset, Point2::new(2, 3) + offset);
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
                let bbi = Bounds2::new(Point2::new(xmin, ymin), Point2::new(xmax, ymax));
                assert_eq!(bb0.intersection(bb1).unwrap(), bbi);
            }
        }
        let bb0 = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        let bb1 = Bounds2::new(Point2::new(-1, -1), Point2::new(2, 2));
        assert_eq!(bb0.intersection(bb1).unwrap(), bb0);
        assert_eq!(bb1.intersection(bb0).unwrap(), bb0);
        let bb0 = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        let bb1 = Bounds2::new(Point2::new(2, 2), Point2::new(3, 3));
        assert!(bb0.intersection(bb1).is_none());

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(Point3::new(0, 1, 2), Point3::new(3, 4, 5));
        let bb1 = Bounds3::new(Point3::new(-1, -2, -3), Point3::new(1, 2, 3));
        assert_eq!(
            bb0.intersection(bb1).unwrap(),
            Bounds3::new(Point3::new(0, 1, 2), Point3::new(1, 2, 3))
        );
    }

    #[test]
    fn expanded() {
        let bb0 = Bounds2::new(Point2::new(0, 1), Point2::new(2, 3));
        assert_eq!(
            bb0.expanded(1).unwrap(),
            Bounds2::new(Point2::new(-1, 0), Point2::new(3, 4))
        );
        assert_eq!(
            bb0.expanded(-1).unwrap(),
            Bounds2::new(Point2::new(1, 2), Point2::new(1, 2))
        );
        assert!(Bounds2::new(Point2::new(0, 0), Point2::new(1, 4))
            .expanded(-2)
            .is_none());
        assert!(Bounds2::new(Point2::new(0, 0), Point2::new(4, 1))
            .expanded(-2)
            .is_none());

        let bb0 = Bounds3::new(Point3::new(0, 1, 2), Point3::new(3, 4, 5));
        assert_eq!(
            bb0.expanded(1).unwrap(),
            Bounds3::new(Point3::new(-1, 0, 1), Point3::new(4, 5, 6))
        );
        assert_eq!(
            bb0.expanded(-1).unwrap(),
            Bounds3::new(Point3::new(1, 2, 3), Point3::new(2, 3, 4))
        );
    }

    #[test]
    fn overlaps() {
        let bb0 = Bounds2::new(Point2::new(0, 1), Point2::new(2, 3));
        for j in -1..=1 {
            for i in -1..=1 {
                let offset = Point2::new(i, j);
                let bb1 = Bounds2::new(Point2::new(0, 1) + offset, Point2::new(2, 3) + offset);
                assert!(bb0.overlaps(bb1));
            }
        }
        let bb0 = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        let bb1 = Bounds2::new(Point2::new(-1, -1), Point2::new(2, 2));
        assert!(bb0.overlaps(bb1));
        assert!(bb1.overlaps(bb0));
        let bb0 = Bounds2::new(Point2::new(0, 0), Point2::new(1, 1));
        let bb1 = Bounds2::new(Point2::new(2, 2), Point2::new(3, 3));
        assert!(!bb0.overlaps(bb1));

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(Point3::new(0, 1, 2), Point3::new(3, 4, 5));
        let bb1 = Bounds3::new(Point3::new(-1, -2, -3), Point3::new(1, 2, 3));
        assert!(bb0.overlaps(bb1));
    }

    #[test]
    fn inside() {
        let bb0 = Bounds2::new(Point2::new(0, 2), Point2::new(3, 5));
        for j in 2..=5 {
            for i in 0..=3 {
                assert!(bb0.inside(Point2::new(i, j)));
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
            assert!(!bb0.inside(Point2::new(i, j)));
        }

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(Point3::new(0, 1, 2), Point3::new(3, 4, 5));
        assert!(bb0.inside(Point3::new(1, 2, 3)));
    }

    #[test]
    fn inside_exclusive() {
        let bb0 = Bounds2::new(Point2::new(0, 2), Point2::new(3, 5));
        for j in 2..5 {
            for i in 0..3 {
                assert!(bb0.inside_exclusive(Point2::new(i, j)));
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
            assert!(!bb0.inside_exclusive(Point2::new(i, j)));
        }

        // Let's assume stuff works since the 2d case goes through the edge cases
        let bb0 = Bounds3::new(Point3::new(0, 1, 2), Point3::new(3, 4, 5));
        assert!(bb0.inside_exclusive(Point3::new(1, 2, 3)));
    }

    #[test]
    fn diagonal() {
        let bb = Bounds2::new(Point2::new(1, 2), Point2::new(4, 7));
        assert_eq!(bb.diagonal(), Vec2::new(3, 5));
        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(5, 8, 11));
        assert_eq!(bb.diagonal(), Vec3::new(4, 6, 8));
    }

    #[test]
    fn lerp() {
        let p0 = Point2::new(1, 2);
        let p1 = Point2::new(3, 4);
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

        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(4, 5, 6));
        // Tests both existence of the func and rounding
        assert_eq!(bb.lerp(Point3::new(0.5, 0.5, 0.5)), Point3::new(2, 3, 4));
    }

    #[test]
    fn offset() {
        let p0 = Point2::new(1.0, 2.0);
        let p1 = Point2::new(3.0, 4.0);
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

        let bb = Bounds3::new(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));
        // Tests both existence of the func and rounding
        assert_eq!(
            bb.offset(Point3::new(2.5, 3.5, 4.5)),
            Vec3::new(0.5, 0.5, 0.5)
        );
    }

    #[test]
    fn area() {
        let bb = Bounds2::new(Point2::new(1, 2), Point2::new(3, 5));
        assert_eq!(bb.area(), 6);
        let bb = Bounds2::new(Point2::new(-1, -2), Point2::new(-3, -5));
        assert_eq!(bb.area(), 6);
    }

    #[test]
    fn surface_area() {
        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(3, 5, 7));
        assert_eq!(bb.surface_area(), 52);
        let bb = Bounds3::new(Point3::new(-1, -2, -3), Point3::new(-3, -5, -7));
        assert_eq!(bb.surface_area(), 52);
    }

    #[test]
    fn volume() {
        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(3, 5, 7));
        assert_eq!(bb.volume(), 24);
        let bb = Bounds3::new(Point3::new(-1, -2, -3), Point3::new(-3, -5, -7));
        assert_eq!(bb.volume(), 24);
    }

    #[test]
    fn maximum_extent() {
        let bb = Bounds2::new(Point2::new(1, 2), Point2::new(3, 5));
        assert_eq!(bb.maximum_extent(), 1);
        let bb = Bounds2::new(Point2::new(1, 2), Point2::new(4, 3));
        assert_eq!(bb.maximum_extent(), 0);

        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(4, 5, 7));
        assert_eq!(bb.maximum_extent(), 2);
        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(4, 6, 5));
        assert_eq!(bb.maximum_extent(), 1);
        let bb = Bounds3::new(Point3::new(1, 2, 3), Point3::new(7, 5, 6));
        assert_eq!(bb.maximum_extent(), 0);
    }

    #[test]
    fn bounding_sphere() {
        let p0 = Point3::new(1.0, 2.0, 3.0);
        let p1 = Point3::new(4.0, 5.0, 6.0);
        let center = Point3::new(2.5, 3.5, 4.5);
        let radius = center.dist(p1);
        let bb = Bounds3::new(p0, p1);
        assert_eq!(bb.bounding_sphere().unwrap(), (center, radius));

        let bb = Bounds3 {
            p_min: Point3::new(5.0, 6.0, 7.0),
            p_max: p1,
        };
        assert!(bb.bounding_sphere().is_none());
    }

    #[test]
    fn iter() {
        let bb = Bounds2::new(Point2::new(-1, -2), Point2::new(3, 4));
        let mut bb_iter = bb.into_iter();
        for j in -2..4 {
            for i in -1..3 {
                assert_eq!(bb_iter.next().unwrap(), Point2::new(i, j));
            }
        }
        assert!(bb_iter.next().is_none());
    }
}
