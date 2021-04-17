#[cfg(test)]
mod tests {
    use approx::{abs_diff_eq, assert_abs_diff_eq, relative_eq};
    use std::panic;

    use yuki::math::{Point2, Point3, Vec2, Vec3};

    // Test the Point* specific methods and merely the existence of methods shared
    // with Vec* since vector tests already validate permutations for those
    // Aim is to check everything we expect is implemented and works as expected.

    #[test]
    fn new() {
        let p = Point2::new(0.0, 1.0);
        assert_eq!(Point2::new(0.0, 1.0), p);

        let p = Point3::new(0.0, 1.0, 2.0);
        assert_eq!(p.z, 2.0);
        assert_eq!(Point3::new(0.0, 1.0, 2.0), p)
    }

    #[test]
    fn nan() {
        assert!(!Point2::new(0, 0).has_nans());
        assert!(!Point3::new(0, 0, 0).has_nans());

        // New
        let result = panic::catch_unwind(|| Point2::new(f32::NAN, 0.0));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| Point3::new(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());

        // Shorthand constructors
        let result = panic::catch_unwind(|| Point2::new(f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Point3::new(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());
    }

    #[test]
    fn dist() {
        let p0 = Point2::new(1.0, 2.0);
        let p1 = p0 + (Vec2::new(3.0, 4.0).normalized() * 3.0);
        assert_abs_diff_eq!(p0.dist(p1), 3.0);

        let p0 = Point3::new(1.0, 2.0, 3.0);
        let p1 = p0 + (Vec3::new(4.0, 5.0, 6.0).normalized() * 3.0);
        assert_abs_diff_eq!(p0.dist(p1), 3.0);
    }

    #[test]
    fn dist_sqr() {
        let p0 = Point2::new(1.0, 2.0);
        let p1 = p0 + (Vec2::new(3.0, 4.0).normalized() * 3.0);
        assert_abs_diff_eq!(p0.dist_sqr(p1), 9.0);

        let p0 = Point3::new(1.0, 2.0, 3.0);
        let p1 = p0 + (Vec3::new(4.0, 5.0, 6.0).normalized() * 3.0);
        assert_abs_diff_eq!(p0.dist_sqr(p1), 9.0);
    }

    #[test]
    fn lerp() {
        let p0 = Point2::new(1.0, 2.0);
        let p1 = Point2::new(4.0, 8.0);
        assert_abs_diff_eq!(p0.lerp(p1, 0.0), Point2::new(1.0, 2.0));
        assert_abs_diff_eq!(p0.lerp(p1, 0.5), Point2::new(2.5, 5.0));
        assert_abs_diff_eq!(p0.lerp(p1, 1.0), Point2::new(4.0, 8.0));

        let p0 = Point3::new(1.0, 2.0, 4.0);
        let p1 = Point3::new(4.0, 8.0, 16.0);
        assert_abs_diff_eq!(p0.lerp(p1, 0.0), Point3::new(1.0, 2.0, 4.0));
        assert_abs_diff_eq!(p0.lerp(p1, 0.5), Point3::new(2.5, 5.0, 10.0));
        assert_abs_diff_eq!(p0.lerp(p1, 1.0), Point3::new(4.0, 8.0, 16.0));
    }

    #[test]
    fn floor() {
        assert_eq!(Point2::new(1.5, 2.5).floor(), Point2::new(1.0, 2.0));
        assert_eq!(
            Point3::new(1.5, 2.5, 3.5).floor(),
            Point3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn ceil() {
        assert_eq!(Point2::new(1.5, 2.5).ceil(), Point2::new(2.0, 3.0));
        assert_eq!(
            Point3::new(1.5, 2.5, 3.5).ceil(),
            Point3::new(2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn abs() {
        assert_eq!(Point2::new(-1, -1).abs(), Point2::new(1, 1));
        assert_eq!(Point3::new(-1, -1, -1).abs(), Point3::new(1, 1, 1));
        assert_eq!(Point2::new(1, 1).abs(), Point2::new(1, 1));
        assert_eq!(Point3::new(1, 1, 1).abs(), Point3::new(1, 1, 1));
    }

    #[test]
    fn min() {
        let a = Point2::new(0, 2);
        let b = Point2::new(3, 1);
        assert_eq!(a.min(b), Point2::new(0, 1));

        let a = Point3::new(0, 2, 4);
        let b = Point3::new(3, 1, 5);
        assert_eq!(a.min(b), Point3::new(0, 1, 4));
    }

    #[test]
    fn max() {
        let a = Point2::new(0, 2);
        let b = Point2::new(3, 1);
        assert_eq!(a.max(b), Point2::new(3, 2));

        let a = Point3::new(0, 2, 4);
        let b = Point3::new(3, 1, 5);
        assert_eq!(a.max(b), Point3::new(3, 2, 5));
    }

    #[test]
    fn permutation() {
        assert_eq!(Point2::new(2.0, 3.0).permuted(1, 0), Point2::new(3.0, 2.0));
        assert_eq!(
            Point3::new(3.0, 4.0, 5.0).permuted(1, 2, 0),
            Point3::new(4.0, 5.0, 3.0)
        );
    }

    #[test]
    fn index() {
        let p = Point2::new(0.0, 1.0);
        assert_eq!(p.x, p[0]);

        let p = Point3::new(0.0, 1.0, 2.0);
        assert_eq!(p.x, p[0]);

        let mut p = Point2::zeros();
        p[0] = 1.0;

        let mut p = Point3::zeros();
        p[0] = 1.0;
    }

    #[test]
    fn add() {
        assert_eq!(Point2::new(1, 2) + Point2::new(4, 6), Point2::new(5, 8));
        assert_eq!(Point2::new(1, 2) + Vec2::new(4, 6), Point2::new(5, 8));
        assert_eq!(
            Point3::new(1, 2, 3) + Point3::new(4, 6, 7),
            Point3::new(5, 8, 10)
        );
        assert_eq!(
            Point3::new(1, 2, 3) + Vec3::new(4, 6, 7),
            Point3::new(5, 8, 10)
        );
    }

    #[test]
    fn sub() {
        assert_eq!(Point2::new(5, 5) - Vec2::new(1, 2), Point2::new(4, 3));
        assert_eq!(Point2::new(5, 5) - Point2::new(1, 2), Vec2::new(4, 3));
        assert_eq!(
            Point3::new(7, 7, 7) - Vec3::new(1, 2, 3),
            Point3::new(6, 5, 4)
        );
        assert_eq!(
            Point3::new(7, 7, 7) - Point3::new(1, 2, 3),
            Vec3::new(6, 5, 4)
        );
    }

    #[test]
    fn mul() {
        assert_eq!(Point2::new(2, 3) * 4, Point2::new(8, 12));
        assert_eq!(Point3::new(2, 3, 4) * 5, Point3::new(10, 15, 20));
    }

    #[test]
    fn add_assign() {
        let mut v = Point2::new(1, 2);
        v += Point2::new(4, 6);
        assert_eq!(v, Point2::new(5, 8));

        let mut v = Point3::new(1, 2, 3);
        v += Point3::new(4, 6, 7);
        assert_eq!(v, Point3::new(5, 8, 10));
    }

    #[test]
    fn mul_assign() {
        let mut p = Point2::new(2, 3);
        p *= 4;
        assert_eq!(p, Point2::new(8, 12));

        let mut p = Point3::new(2, 3, 4);
        p *= 5;
        assert_eq!(p, Point3::new(10, 15, 20));
    }

    #[test]
    fn abs_diff_eq() {
        assert!(abs_diff_eq!(
            &Point2::<f32>::zeros(),
            &Point2::<f32>::zeros()
        ));
    }

    #[test]
    fn relative_eq() {
        assert!(relative_eq!(
            &Point2::<f32>::zeros(),
            &Point2::<f32>::zeros()
        ));
    }
}
