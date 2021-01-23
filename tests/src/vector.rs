#[cfg(test)]
mod tests {
    use approx::{assert_abs_diff_eq, assert_abs_diff_ne, assert_relative_eq, assert_relative_ne};
    use std::panic;

    use yuki::math::{
        normal::Normal,
        point::Point3,
        vector::{vec2, vec3, vec4, Vec2, Vec3, Vec4},
    };

    // Test both Vec* structs and the generation macros here.
    // Aim is to check everything we expect is implemented and works as expected.

    // Use Vec2 as the permutation check for brevity. The impls are expanded using
    // the concrete components of the type so any vector length/naming convention
    // should work if one works.

    // It makes no sense to test impl evaluation here with dummy types since we
    // depend on the real types and they need to be compiled before.

    #[test]
    fn new() {
        let v = Vec2::new(0.0, 1.0);
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 1.0);
        assert_eq!(vec2(0.0, 1.0), v);

        let v = Vec3::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 1.0);
        assert_eq!(v.z, 2.0);
        assert_eq!(vec3(0.0, 1.0, 2.0), v);

        let v = Vec4::new(0.0, 1.0, 2.0, 3.0);
        assert_eq!(v.x, 0.0f32);
        assert_eq!(v.y, 1.0f32);
        assert_eq!(v.z, 2.0f32);
        assert_eq!(v.w, 3.0f32);
        assert_eq!(vec4(0.0, 1.0, 2.0, 3.0), v);
    }

    #[test]
    fn zeros() {
        assert_eq!(Vec2::zeros(), Vec2::new(0, 0));
        assert_eq!(Vec3::zeros(), Vec3::new(0, 0, 0));
        assert_eq!(Vec4::zeros(), Vec4::new(0, 0, 0, 0));
    }

    #[test]
    fn ones() {
        assert_eq!(Vec2::ones(), Vec2::new(1, 1));
        assert_eq!(Vec3::ones(), Vec3::new(1, 1, 1));
        assert_eq!(Vec4::ones(), Vec4::new(1, 1, 1, 1));
    }

    #[test]
    fn has_nans() {
        // Test all permutations with constructor as it should panic
        let result = panic::catch_unwind(|| Vec2::new(f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| Vec2::new(0.0, f32::NAN));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| Vec3::new(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| Vec4::new(f32::NAN, 0.0, 0.0, 0.0));
        assert!(result.is_err());

        // Test shorthand constructors
        let result = panic::catch_unwind(|| vec2(f32::NAN, 0.0));
        assert!(result.is_err());
        let result = panic::catch_unwind(|| vec2(0.0, f32::NAN));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| vec3(f32::NAN, 0.0, 0.0));
        assert!(result.is_err());

        let result = panic::catch_unwind(|| vec4(f32::NAN, 0.0, 0.0, 0.0));
        assert!(result.is_err());
    }

    #[test]
    fn dot() {
        assert_eq!(Vec2::new(2, 3).dot(Vec2::new(4, 5)), 2 * 4 + 3 * 5);
        assert_eq!(
            Vec3::new(2, 3, 4).dot(Vec3::new(5, 6, 7)),
            2 * 5 + 3 * 6 + 4 * 7
        );
        assert_eq!(
            Vec4::new(2, 3, 4, 5).dot(Vec4::new(6, 7, 8, 9)),
            2 * 6 + 3 * 7 + 4 * 8 + 5 * 9
        );

        assert_eq!(
            Vec3::new(2.0, 3.0, 4.0).dot_n(Normal::new(5.0, 6.0, 7.0)),
            2.0 * 5.0 + 3.0 * 6.0 + 4.0 * 7.0
        );
    }

    #[test]
    fn cross() {
        assert_eq!(
            Vec3::new(2.0, 3.0, 4.0).cross(Vec3::new(5.0, 6.0, -7.0)),
            Vec3::new(-45.0, 34.0, -3.0)
        );
    }

    #[test]
    fn len_sqr() {
        assert_eq!(Vec2::new(2, 3).len_sqr(), 2 * 2 + 3 * 3);
        assert_eq!(Vec3::new(2, 3, 4).len_sqr(), 2 * 2 + 3 * 3 + 4 * 4);
        assert_eq!(
            Vec4::new(2, 3, 4, 5).len_sqr(),
            2 * 2 + 3 * 3 + 4 * 4 + 5 * 5
        );
    }

    #[test]
    fn len() {
        assert_abs_diff_eq!(
            Vec2::new(2.0, 3.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32).sqrt()
        );
        assert_abs_diff_eq!(
            Vec3::new(2.0, 3.0, 4.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32 + 4.0f32 * 4.0f32).sqrt()
        );
        assert_abs_diff_eq!(
            Vec4::new(2.0, 3.0, 4.0, 5.0).len(),
            (2.0f32 * 2.0f32 + 3.0f32 * 3.0f32 + 4.0f32 * 4.0f32 + 5.0f32 * 5.0f32).sqrt()
        );
    }

    #[test]
    fn normalized() {
        assert_abs_diff_eq!(Vec2::new(1.0, 1.0).normalized().len(), 1.0);
        assert_abs_diff_eq!(Vec3::new(1.0, 1.0, 1.0).normalized().len(), 1.0);
        assert_abs_diff_eq!(Vec4::new(1.0, 1.0, 1.0, 1.0).normalized().len(), 1.0);
    }

    #[test]
    fn min() {
        let a = Vec2::new(0, 2);
        let b = Vec2::new(3, 1);
        assert_eq!(a.min(b), Vec2::new(0, 1));
        assert_eq!(a.min(b), b.min(a));

        let a = Vec3::new(0, 2, 4);
        let b = Vec3::new(3, 1, 5);
        assert_eq!(a.min(b), Vec3::new(0, 1, 4));
        assert_eq!(a.min(b), b.min(a));

        let a = Vec4::new(0, 2, 4, 7);
        let b = Vec4::new(3, 1, 5, 6);
        assert_eq!(a.min(b), Vec4::new(0, 1, 4, 6));
        assert_eq!(a.min(b), b.min(a));
    }

    #[test]
    fn max() {
        let a = Vec2::new(0, 2);
        let b = Vec2::new(3, 1);
        assert_eq!(a.max(b), Vec2::new(3, 2));
        assert_eq!(a.max(b), b.max(a));

        let a = Vec3::new(0, 2, 4);
        let b = Vec3::new(3, 1, 5);
        assert_eq!(a.max(b), Vec3::new(3, 2, 5));
        assert_eq!(a.max(b), b.max(a));

        let a = Vec4::new(0, 2, 4, 7);
        let b = Vec4::new(3, 1, 5, 6);
        assert_eq!(a.max(b), Vec4::new(3, 2, 5, 7));
        assert_eq!(a.max(b), b.max(a));
    }

    #[test]
    fn min_comp() {
        assert_eq!(Vec2::new(0.0, 1.0).min_comp(), 0.0);
        assert_eq!(Vec2::new(1.0, 0.0).min_comp(), 0.0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).min_comp(), 0.0);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).min_comp(), 0.0);
    }

    #[test]
    fn max_comp() {
        assert_eq!(Vec2::new(0.0, 1.0).max_comp(), 1.0);
        assert_eq!(Vec2::new(1.0, 0.0).max_comp(), 1.0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).max_comp(), 2.0);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).max_comp(), 3.0);
    }

    #[test]
    fn max_dimension() {
        assert_eq!(Vec2::new(0.0, 1.0).max_dimension(), 1);
        assert_eq!(Vec2::new(1.0, 0.0).max_dimension(), 0);

        assert_eq!(Vec3::new(0.0, 1.0, 2.0).max_dimension(), 2);

        assert_eq!(Vec4::new(0.0, 1.0, 2.0, 3.0).max_dimension(), 3);
    }

    #[test]
    fn permutation() {
        assert_eq!(Vec2::new(2.0, 3.0).permuted(1, 0), Vec2::new(3.0, 2.0));
        assert_eq!(
            Vec3::new(3.0, 4.0, 5.0).permuted(1, 2, 0),
            Vec3::new(4.0, 5.0, 3.0)
        );
        assert_eq!(
            Vec4::new(4.0, 5.0, 6.0, 7.0).permuted(1, 2, 3, 0),
            Vec4::new(5.0, 6.0, 7.0, 4.0)
        );
    }

    #[test]
    fn from() {
        assert_eq!(Vec2::from(2), Vec2::new(2, 2));
        assert_eq!(Vec3::from(2), Vec3::new(2, 2, 2));
        assert_eq!(Vec4::from(2), Vec4::new(2, 2, 2, 2));

        assert_eq!(
            Vec3::from(Normal::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(
            Vec3::from(Point3::new(1.0, 2.0, 3.0)),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn index() {
        let v = Vec2::new(0.0, 1.0);
        assert_eq!(v.x, v[0]);
        assert_eq!(v.y, v[1]);

        let v = Vec3::new(0.0, 1.0, 2.0);
        assert_eq!(v.x, v[0]);

        let v = Vec4::new(0.0, 1.0, 2.0, 3.0);
        assert_eq!(v.x, v[0]);

        let mut v = Vec2::zeros();
        v[0] = 1.0;
        v[1] = 2.0;
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);

        let mut v = Vec3::zeros();
        v[0] = 1.0;
        assert_eq!(v[0], 1.0);

        let mut v = Vec4::zeros();
        v[0] = 1.0;
        assert_eq!(v[0], 1.0);
    }

    #[test]
    fn neg() {
        assert_eq!(-Vec2::new(1, 2), Vec2::new(-1, -2));
        assert_eq!(-Vec3::new(1, 2, 3), Vec3::new(-1, -2, -3));
        assert_eq!(-Vec4::new(1, 2, 3, 4), Vec4::new(-1, -2, -3, -4));
    }

    #[test]
    fn add() {
        // Vec + Vec
        assert_eq!(Vec2::new(1, 2) + Vec2::new(4, 6), Vec2::new(5, 8));
        assert_eq!(Vec3::new(1, 2, 3) + Vec3::new(4, 6, 7), Vec3::new(5, 8, 10));
        assert_eq!(
            Vec4::new(1, 2, 3, 4) + Vec4::new(5, 7, 9, 10),
            Vec4::new(6, 9, 12, 14)
        );

        // Vec + Scalar
        assert_eq!(Vec2::new(1, 2) + 3, Vec2::new(4, 5));
        assert_eq!(Vec3::new(1, 2, 3) + 4, Vec3::new(5, 6, 7));
        assert_eq!(Vec4::new(1, 2, 3, 4) + 5, Vec4::new(6, 7, 8, 9));
    }

    #[test]
    fn sub() {
        // Vec - Vec
        assert_eq!(Vec2::new(5, 5) - Vec2::new(1, 2), Vec2::new(4, 3));
        assert_eq!(Vec3::new(7, 7, 7) - Vec3::new(1, 2, 3), Vec3::new(6, 5, 4));
        assert_eq!(
            Vec4::new(9, 9, 9, 9) - Vec4::new(1, 2, 3, 4),
            Vec4::new(8, 7, 6, 5)
        );

        // Vec - Scalar
        assert_eq!(Vec2::new(3, 2) - 2, Vec2::new(1, 0));
        assert_eq!(Vec3::new(7, 6, 5) - 4, Vec3::new(3, 2, 1));
        assert_eq!(Vec4::new(9, 8, 7, 6) - 5, Vec4::new(4, 3, 2, 1));
    }

    #[test]
    fn mul() {
        // Vec * Scalar
        assert_eq!(Vec2::new(2, 3) * 4, Vec2::new(8, 12));
        assert_eq!(Vec3::new(2, 3, 4) * 5, Vec3::new(10, 15, 20));
        assert_eq!(Vec4::new(2, 3, 4, 5) * 6, Vec4::new(12, 18, 24, 30));
    }

    #[test]
    fn div() {
        // Vec / Scalar
        assert_eq!(Vec2::new(8, 12) / 4, Vec2::new(2, 3));
        assert_eq!(Vec3::new(10, 15, 20) / 5, Vec3::new(2, 3, 4));
        assert_eq!(Vec4::new(12, 18, 24, 30) / 6, Vec4::new(2, 3, 4, 5));
    }

    #[test]
    fn add_assign() {
        // Vec += Vec
        let mut v = Vec2::new(1, 2);
        v += Vec2::new(4, 6);
        assert_eq!(v, Vec2::new(5, 8));

        let mut v = Vec3::new(1, 2, 3);
        v += Vec3::new(4, 6, 7);
        assert_eq!(v, Vec3::new(5, 8, 10));

        let mut v = Vec4::new(1, 2, 3, 4);
        v += Vec4::new(5, 7, 9, 10);
        assert_eq!(v, Vec4::new(6, 9, 12, 14));

        // Vec += Scalar
        let mut v = Vec2::new(1, 2);
        v += 3;
        assert_eq!(v, Vec2::new(4, 5));

        let mut v = Vec3::new(1, 2, 3);
        v += 4;
        assert_eq!(v, Vec3::new(5, 6, 7));

        let mut v = Vec4::new(1, 2, 3, 4);
        v += 5;
        assert_eq!(v, Vec4::new(6, 7, 8, 9));
    }

    #[test]
    fn sub_assign() {
        // Vec -= Vec
        let mut v = Vec2::new(5, 5);
        v -= Vec2::new(1, 2);
        assert_eq!(v, Vec2::new(4, 3));

        let mut v = Vec3::new(7, 7, 7);
        v -= Vec3::new(1, 2, 3);
        assert_eq!(v, Vec3::new(6, 5, 4));

        let mut v = Vec4::new(9, 9, 9, 9);
        v -= Vec4::new(1, 2, 3, 4);
        assert_eq!(v, Vec4::new(8, 7, 6, 5));

        // Vec -= Scalar
        let mut v = Vec2::new(3, 2);
        v -= 2;
        assert_eq!(v, Vec2::new(1, 0));

        let mut v = Vec3::new(7, 6, 5);
        v -= 4;
        assert_eq!(v, Vec3::new(3, 2, 1));

        let mut v = Vec4::new(9, 8, 7, 6);
        v -= 5;
        assert_eq!(v, Vec4::new(4, 3, 2, 1));
    }

    #[test]
    fn mul_assign() {
        // Vec *= Scalar
        let mut v = Vec2::new(2, 3);
        v *= 4;
        assert_eq!(v, Vec2::new(8, 12));

        let mut v = Vec3::new(2, 3, 4);
        v *= 5;
        assert_eq!(v, Vec3::new(10, 15, 20));

        let mut v = Vec4::new(2, 3, 4, 5);
        v *= 6;
        assert_eq!(v, Vec4::new(12, 18, 24, 30));
    }

    #[test]
    fn div_assign() {
        // Vec /= Scalar
        let mut v = Vec2::new(8, 12);
        v /= 4;
        assert_eq!(v, Vec2::new(2, 3));

        let mut v = Vec3::new(10, 15, 20);
        v /= 5;
        assert_eq!(v, Vec3::new(2, 3, 4));

        let mut v = Vec4::new(12, 18, 24, 30);
        v /= 6;
        assert_eq!(v, Vec4::new(2, 3, 4, 5));
    }

    #[test]
    fn abs_diff_eq() {
        // Basic cases
        assert_abs_diff_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::zeros());
        assert_abs_diff_ne!(&Vec2::<f32>::zeros(), &Vec2::<f32>::ones());

        // Should fail on diff in any coordinate if no epsilon is given
        assert_abs_diff_ne!(&Vec2::new(0.0, 1.0), &Vec2::zeros());
        assert_abs_diff_ne!(&Vec2::new(1.0, 0.0), &Vec2::zeros());

        // Should fail on diff in any coordinate that doesn't fit epsilon
        assert_abs_diff_ne!(&Vec2::new(0.0, 1.0), &Vec2::zeros());
        assert_abs_diff_ne!(&Vec2::new(1.0, 0.0), &Vec2::zeros());

        // Should succeed with matching epsilon
        assert_abs_diff_eq!(&Vec2::new(1.0, 1.0), &Vec2::zeros(), epsilon = 1.0);
    }

    #[test]
    fn relative_eq() {
        // Basic cases
        assert_relative_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::zeros());
        assert_relative_ne!(&Vec2::<f32>::zeros(), &Vec2::<f32>::ones());

        // Should fail on diff in any coordinate if no epsilon is given
        assert_relative_ne!(&Vec2::new(0.0, 1.0), &Vec2::zeros());
        assert_relative_ne!(&Vec2::new(1.0, 0.0), &Vec2::zeros());

        // Should fail on diff in any coordinate that doesn't fit epsilon or max_relative
        assert_relative_ne!(&Vec2::new(0.0, 1.0), &Vec2::zeros());
        assert_relative_ne!(&Vec2::new(1.0, 0.0), &Vec2::zeros());

        // Should succeed with matching epsilon
        assert_relative_eq!(&Vec2::new(1.0, 1.0), &Vec2::zeros(), epsilon = 1.0,);

        // Should succeed with diff that fits max_relative
        assert_relative_eq!(
            &Vec2::new(2.0, 2.0),
            &Vec2::ones(),
            epsilon = 0.0,
            max_relative = 0.5
        );
    }
}
