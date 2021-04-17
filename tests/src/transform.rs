#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use yuki::math::{Bounds3, Matrix4x4, Normal, Point3, Ray, Transform, Vec3};

    // These are by no means exhaustive. We throw some simple cases at the implementation
    // to catch obvious typos

    #[test]
    fn new() {
        let md = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let m = Matrix4x4::new(md);
        let mi = m.inverted();

        let t0 = Transform::new(md);
        let t1 = Transform::new_m(m);
        let t2 = Transform::new_full(m, mi);
        assert_eq!(t0.m(), &m);
        assert_eq!(t0.m_inv(), &mi);
        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
    }

    #[test]
    fn default() {
        let t = Transform::default();
        let m = Matrix4x4::<f32>::identity();
        let ti = Transform::new_full(m, m);
        assert_eq!(t, ti);
    }

    #[test]
    fn inverted() {
        let m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let t = Transform::new_m(m);
        let ti = t.inverted();
        assert_eq!(t.m(), ti.m_inv());
        assert_eq!(t.m_inv(), ti.m());
    }

    #[test]
    fn transposed() {
        let m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let t = Transform::new_m(m).transposed();
        assert_eq!(t.m(), &m.transposed());
        assert_eq!(t.m_inv(), &m.inverted().transposed());
    }

    #[test]
    fn is_identity() {
        let t = Transform::new_m(Matrix4x4::<f32>::identity());
        assert!(t.is_identity());

        let t = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        assert!(!t.is_identity());
    }

    #[test]
    fn swaps_handedness() {
        let t = Transform::new_m(Matrix4x4::<f32>::identity());
        assert!(!t.swaps_handedness());

        let t = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        assert!(t.swaps_handedness());

        let t = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        assert!(t.swaps_handedness());
    }

    #[test]
    fn mul() {
        let t = Transform::new([
            [16.0, 11.0, 6.0, 13.0],
            [12.0, 15.0, 10.0, 9.0],
            [8.0, 7.0, 14.0, 5.0],
            [4.0, 3.0, 2.0, 1.0],
        ]);
        let tp = Transform::new([
            [16.0, 11.0, 6.0, 13.0],
            [12.0, 15.0, 10.0, 9.0],
            [8.0, 7.0, 14.0, 5.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        let v = Vec3::new(17.0, 18.0, 19.0);
        assert_eq!(&t * v, Vec3::new(584.0, 664.0, 528.0));

        let p = Point3::new(17.0, 18.0, 19.0);
        assert_eq!(&t * p, Point3::new(597.0, 673.0, 533.0) / 161.0);
        assert_eq!(&tp * p, Point3::new(597.0, 673.0, 533.0));

        let n = Normal::<f32>::new(17.0, 18.0, 19.0);
        assert_eq!(&t * n, Normal::new(-1.0694447, 0.5972222, 0.5972223));

        let r = Ray::new(p, Vec3::new(20.0, 21.0, 22.0), 23.0);
        assert_eq!(
            &t * r,
            Ray::new(
                Point3::new(597.0, 673.0, 533.0) / 161.0,
                Vec3::new(683.0, 775.0, 615.0),
                23.0
            )
        );

        let bb0 = Bounds3::new(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));
        let corners = [
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(4.0, 2.0, 3.0),
            Point3::new(4.0, 5.0, 3.0),
            Point3::new(1.0, 5.0, 3.0),
            Point3::new(1.0, 2.0, 6.0),
            Point3::new(4.0, 2.0, 6.0),
            Point3::new(1.0, 5.0, 6.0),
            Point3::new(4.0, 5.0, 6.0),
        ];
        let mut bb1 = &t * Bounds3::new(corners[0], corners[0]);
        for &c in &corners {
            bb1 = bb1.union_b(&t * Bounds3::new(c, c));
        }
        assert_eq!(&t * bb0, bb1);

        let ttpm = Transform::new_m(t.m() * tp.m());
        let ttpt = &t * &tp;
        assert_eq!(ttpm.m(), ttpt.m());
        assert_abs_diff_eq!(ttpm.m_inv(), ttpt.m_inv(), epsilon = 3e-6);
    }

    #[test]
    fn translation() {
        let tm = Matrix4x4::new([
            [1.0, 0.0, 0.0, 2.0],
            [0.0, 1.0, 0.0, 3.0],
            [0.0, 0.0, 1.0, 4.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let tt = yuki::math::transforms::translation(Vec3::new(2.0, 3.0, 4.0));
        assert_eq!(tt.m(), &tm);
        assert_eq!(tt.m_inv(), &tm.inverted());
    }

    #[test]
    fn scale() {
        let sm = Matrix4x4::new([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let st = yuki::math::transforms::scale(2.0, 3.0, 4.0);
        assert_eq!(st.m(), &sm);
        assert_eq!(st.m_inv(), &sm.inverted());
    }

    #[test]
    fn rotation_x() {
        let rm = Matrix4x4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let rt = yuki::math::transforms::rotation_x(std::f64::consts::FRAC_PI_2);
        assert_abs_diff_eq!(rt.m(), &rm, epsilon = 1e-16);
        assert_abs_diff_eq!(rt.m_inv(), &rm.inverted(), epsilon = 1e-16);
    }

    #[test]
    fn rotation_y() {
        let rm = Matrix4x4::new([
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let rt = yuki::math::transforms::rotation_y(std::f64::consts::FRAC_PI_2);
        assert_abs_diff_eq!(rt.m(), &rm, epsilon = 1e-16);
        assert_abs_diff_eq!(rt.m_inv(), &rm.inverted(), epsilon = 1e-16);
    }

    #[test]
    fn rotation_z() {
        let rm = Matrix4x4::new([
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let rt = yuki::math::transforms::rotation_z(std::f64::consts::FRAC_PI_2);
        assert_abs_diff_eq!(rt.m(), &rm, epsilon = 1e-16);
        assert_abs_diff_eq!(rt.m_inv(), &rm.inverted(), epsilon = 1e-16);
    }

    #[test]
    fn rotation() {
        let rm = Matrix4x4::new([
            [
                0.333333333333333,
                -0.244016935856292,
                0.910683602522959,
                0.0,
            ],
            [
                0.910683602522959,
                0.333333333333333,
                -0.244016935856292,
                0.0,
            ],
            [
                -0.244016935856292,
                0.910683602522959,
                0.333333333333333,
                0.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let rt =
            yuki::math::transforms::rotation(std::f64::consts::FRAC_PI_2, Vec3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(rt.m(), &rm, epsilon = 1e-15);
        assert_abs_diff_eq!(rt.m_inv(), &rm.inverted(), epsilon = 1e-15);
    }

    #[test]
    fn look_at() {
        let m = Matrix4x4::new([
            [
                0.825307261249832,
                -0.322265731783557,
                0.463694643754174,
                1.0,
            ],
            [0.0, 0.821157874256179, 0.570701100005137, 2.0],
            [
                -0.564683915591990,
                -0.471003761837506,
                0.677707556256101,
                3.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        let mt = yuki::math::transforms::look_at(
            Point3::new(1.0, 2.0, 3.0),
            Point3::new(40.0, 50.0, 60.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // assert_abs_diff_eq!(mt.m(), &m.inverted(), epsilon = 1e-15);
        assert_abs_diff_eq!(mt.m_inv(), &m, epsilon = 1e-15);
    }
}
