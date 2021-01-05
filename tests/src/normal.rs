#[cfg(test)]
mod tests {
    use approx::{abs_diff_eq, assert_abs_diff_eq, relative_eq};
    use std::panic;
    use yuki::normal::{normal, Normal};
    use yuki::vector::Vec3;

    // Test the Normal specific methods and merely the existence of methods shared
    // with Vec* since vector tests already validate permutations for those
    // Aim is to check everything we expect is implemented and works as expected.

    #[test]
    fn new() {
        let v = Normal::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 1.0);
        assert_eq!(v.z, 2.0);
        assert_eq!(normal(0.0, 1.0, 2.0), v);
    }

    #[test]
    fn zeros() {
        assert_eq!(Normal::zeros(), Normal::new(0, 0, 0));
    }

    #[test]
    fn ones() {
        assert_eq!(Normal::ones(), Normal::new(1, 1, 1));
    }

    #[test]
    fn has_nans() {
        // Test constructor as it should panic
        let result = panic::catch_unwind(|| Normal::new(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());

        // Test shorthand constructor
        let result = panic::catch_unwind(|| normal(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());
    }
    #[test]
    fn dot() {
        assert_eq!(
            Normal::new(2, 3, 4).dot(Normal::new(5, 6, 7)),
            2 * 5 + 3 * 6 + 4 * 7
        );
        assert_eq!(
            Normal::new(2, 3, 4).dot_v(Vec3::new(5, 6, 7)),
            2 * 5 + 3 * 6 + 4 * 7
        );
    }

    #[test]
    fn len_sqr() {
        assert_eq!(Normal::new(2, 3, 4).len_sqr(), 2 * 2 + 3 * 3 + 4 * 4);
    }

    #[test]
    fn len() {
        assert_abs_diff_eq!(
            Normal::new(2.0, 3.0, 4.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32 + 4.0f32 * 4.0f32).sqrt()
        );
    }

    #[test]
    fn normalized() {
        assert_abs_diff_eq!(Normal::new(1.0, 1.0, 1.0).normalized().len(), 1.0);
    }

    #[test]
    fn permutation() {
        assert_eq!(
            Normal::new(3.0, 4.0, 5.0).permuted(1, 2, 0),
            Normal::new(4.0, 5.0, 3.0)
        );
    }

    #[test]
    fn from() {
        assert_eq!(Normal::from(Vec3::new(1, 2, 3)), Normal::new(1, 2, 3));
    }

    #[test]
    fn index() {
        let v = Normal::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, v[0]);

        let mut v = Normal::zeros();
        v[0] = 1.0;
        assert_eq!(v[0], 1.0);
    }

    #[test]
    fn neg() {
        assert_eq!(-Normal::new(1, 2, 3), Normal::new(-1, -2, -3));
    }

    #[test]
    fn add() {
        assert_eq!(
            Normal::new(1, 2, 3) + Normal::new(4, 6, 7),
            Normal::new(5, 8, 10)
        );
    }

    #[test]
    fn sub() {
        assert_eq!(
            Normal::new(7, 7, 7) - Normal::new(1, 2, 3),
            Normal::new(6, 5, 4)
        );
    }

    #[test]
    fn mul() {
        assert_eq!(Normal::new(2, 3, 4) * 5, Normal::new(10, 15, 20));
    }

    #[test]
    fn div() {
        assert_eq!(Normal::new(10, 15, 20) / 5, Normal::new(2, 3, 4));
    }

    #[test]
    fn add_assign() {
        let mut v = Normal::new(1, 2, 3);
        v += Normal::new(4, 6, 7);
        assert_eq!(v, Normal::new(5, 8, 10));
    }

    #[test]
    fn sub_assign() {
        let mut v = Normal::new(7, 7, 7);
        v -= Normal::new(1, 2, 3);
        assert_eq!(v, Normal::new(6, 5, 4));
    }

    #[test]
    fn mul_assign() {
        let mut v = Normal::new(2, 3, 4);
        v *= 5;
        assert_eq!(v, Normal::new(10, 15, 20));
    }

    #[test]
    fn div_assign() {
        let mut v = Normal::new(10, 15, 20);
        v /= 5;
        assert_eq!(v, Normal::new(2, 3, 4));
    }

    #[test]
    fn abs_diff_eq() {
        assert!(abs_diff_eq!(
            &Normal::<f32>::zeros(),
            &Normal::<f32>::zeros()
        ));
    }

    #[test]
    fn relative_eq() {
        assert!(relative_eq!(
            &Normal::<f32>::zeros(),
            &Normal::<f32>::zeros()
        ));
    }
}
