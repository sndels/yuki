#[cfg(test)]
mod tests {
    use approx::{
        abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne, assert_relative_eq, assert_relative_ne,
    };
    use std::panic;

    use yuki::point::{point3, Point3};
    use yuki::ray::Ray;
    use yuki::vector::vec3;

    #[test]
    fn new() {
        let o = point3(1.0, 2.0, 3.0);
        let d = vec3(4.0, 5.0, 6.0);
        let t_max = 4.0;
        let r = Ray::new(o, d, t_max);
        assert_eq!(r.o, o);
        assert_eq!(r.d, d);
        assert_eq!(r.t_max, t_max);

        // We won't be able to construct a vec or point with NaNs so let's just check
        // a NaN t_max panics
        let result = panic::catch_unwind(|| Ray::new(o, d, f32::NAN));
        assert!(result.is_err());
    }

    #[test]
    fn default() {
        let r = Ray::default();
        assert_eq!(r.o, Point3::zeros());
        assert_eq!(r.d, vec3(0.0, 1.0, 0.0));
        assert_eq!(r.t_max, f32::INFINITY);
    }

    #[test]
    fn has_nans() {
        let mut r = Ray::default();
        assert!(!r.has_nans());
        r.o[0] = f32::NAN;
        assert!(r.has_nans());
        r.o[0] = 0.0;
        r.d[0] = f32::NAN;
        assert!(r.has_nans());
        r.d[0] = 0.0;
        r.t_max = f32::NAN;
        assert!(r.has_nans());
        r.t_max = f32::INFINITY;
        assert!(!r.has_nans());
    }

    #[test]
    fn point() {
        let o = point3(1.0, 2.0, 3.0);
        let d = vec3(4.0, 5.0, 6.0);
        let r = Ray::new(o, d, 1.0);
        assert_eq!(r.point(1.0), o + d);
        assert_eq!(r.point(2.0), o + d * 2.0);
    }

    #[test]
    fn abs_diff_eq() {
        let o = point3(1.0, 2.0, 3.0);
        let oa = point3(2.0, 3.0, 4.0);
        let d = vec3(4.0, 5.0, 6.0);
        let da = vec3(5.0, 6.0, 7.0);
        let r = Ray::new(o, d, 1.0);
        let rc = r;
        assert_abs_diff_eq!(r, rc);
        assert_abs_diff_ne!(r, Ray::new(oa, d, 1.0));
        assert_abs_diff_ne!(r, Ray::new(o, da, 1.0));
        assert_abs_diff_ne!(r, Ray::new(o, d, 2.0));
        assert_abs_diff_eq!(r, Ray::new(oa, da, 2.0), epsilon = 1.0);
    }

    #[test]
    fn relative_eq() {
        let o = point3(1.0, 2.0, 3.0);
        let oa = point3(2.0, 3.0, 4.0);
        let or = point3(2.0, 4.0, 6.0);
        let d = vec3(4.0, 5.0, 6.0);
        let da = vec3(5.0, 6.0, 7.0);
        let dr = vec3(8.0, 10.0, 12.0);
        let r = Ray::new(o, d, 1.0);
        let rc = r;
        assert_relative_eq!(r, rc);
        assert_relative_ne!(r, Ray::new(oa, d, 1.0));
        assert_relative_ne!(r, Ray::new(o, da, 1.0));
        assert_relative_ne!(r, Ray::new(o, d, 2.0));
        assert_relative_eq!(r, Ray::new(oa, da, 2.0), epsilon = 1.0);
        assert_relative_eq!(r, Ray::new(or, dr, 2.0), epsilon = 0.0, max_relative = 0.5);
    }
}
