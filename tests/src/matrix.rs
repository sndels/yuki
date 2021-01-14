#[cfg(test)]
mod tests {
    use approx::{assert_abs_diff_eq, assert_abs_diff_ne, assert_relative_eq, assert_relative_ne};
    use std::panic;

    use yuki::math::matrix::Matrix4x4;

    #[test]
    fn zeros() {
        assert_eq!(
            Matrix4x4::zeros(),
            Matrix4x4::new([
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ])
        );
    }

    #[test]
    fn identity() {
        assert_eq!(
            Matrix4x4::identity(),
            Matrix4x4::new([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ])
        );
    }

    #[test]
    fn has_nans() {
        // No NaNs shouldn't panic
        Matrix4x4::<f32>::zeros().has_nans();
        // Any position with NaN should panic
        for row in 0..4 {
            for col in 0..4 {
                let mut m = [[0.0; 4]; 4];
                m[row][col] = f32::NAN;
                let result = panic::catch_unwind(|| Matrix4x4::new(m));
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn row() {
        let m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);

        for row in 0..4 {
            let row_first = (row as f32) * 4.0 + 1.0;
            assert_eq!(
                m.row(row),
                [row_first, row_first + 1.0, row_first + 2.0, row_first + 3.0]
            );
        }
    }

    #[test]
    fn row_mut() {
        let mut m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let mut mc = m;

        for row in 0..4 {
            let rm = m.row_mut(row);
            for col in 0..4 {
                rm[col] *= rm[col];
                mc.m[row][col] *= mc.m[row][col];
            }
        }

        for row in 0..4 {
            for col in 0..4 {
                assert_eq!(m.m[row][col], mc.m[row][col]);
            }
        }
    }

    #[test]
    fn col() {
        let m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ])
        .transposed();

        for col in 0..4 {
            let row_first = (col as f32) * 4.0 + 1.0;
            assert_eq!(
                m.col(col),
                [row_first, row_first + 1.0, row_first + 2.0, row_first + 3.0]
            );
        }
    }

    #[test]
    fn col_mut() {
        let mut m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ])
        .transposed();
        let mut mc = m;

        for col in 0..4 {
            let cm = m.col_mut(col);
            for row in 0..4 {
                *cm[row] *= *cm[row];
                mc.m[row][col] *= mc.m[row][col];
            }
        }

        for row in 0..4 {
            for col in 0..4 {
                assert_eq!(m.m[row][col], mc.m[row][col]);
            }
        }
    }

    #[test]
    fn transposed() {
        let m = Matrix4x4::new([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let mt = Matrix4x4::new([
            [1.0, 5.0, 9.0, 13.0],
            [2.0, 6.0, 10.0, 14.0],
            [3.0, 7.0, 11.0, 15.0],
            [4.0, 8.0, 12.0, 16.0],
        ]);
        let mc = m;

        assert_eq!(m.transposed(), mt);

        // m should remain untouched
        assert_eq!(m, mc);
    }

    #[test]
    fn inverted() {
        // Just some random, non-singular matrix
        let m = Matrix4x4::new([
            [9.2f32, 8.1, 8.0, -2.1],
            [-8.3, 16.0, 3.0, 8.0],
            [0.5, 9.3, -4.0, 7.1],
            [3.0, -8.0, 2.0, 10.0],
        ]);
        let mc = m;

        // A^-1^-1 = A
        assert_abs_diff_eq!(m.inverted().inverted(), m, epsilon = 1e-5);
        // A A^-1 = I
        assert_abs_diff_eq!(&m * &m.inverted(), Matrix4x4::identity(), epsilon = 1e-5);

        // m should remain untouched
        assert_eq!(m, mc);
    }

    #[test]
    fn mul() {
        let m = Matrix4x4::new([
            [1.0f32, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let mc = m;

        assert_abs_diff_eq!(
            &m * &m,
            Matrix4x4::new([
                [90.0, 100.0, 110.0, 120.0],
                [202.0, 228.0, 254.0, 280.0],
                [314.0, 356.0, 398.0, 440.0],
                [426.0, 484.0, 542.0, 600.0],
            ])
        );

        // m should remain untouched
        assert_eq!(m, mc);
    }

    #[test]
    fn abs_diff_eq() {
        assert_abs_diff_eq!(Matrix4x4::<f32>::identity(), Matrix4x4::identity());
        for row in 0..4 {
            for col in 0..4 {
                let mut m = Matrix4x4::zeros();
                m.m[row][col] = 1.0;
                assert_abs_diff_ne!(m, Matrix4x4::identity());
                assert_abs_diff_eq!(m, Matrix4x4::identity(), epsilon = 1.0)
            }
        }
    }

    #[test]
    fn relative_eq() {
        assert_abs_diff_eq!(Matrix4x4::<f32>::identity(), Matrix4x4::identity());
        for row in 0..4 {
            for col in 0..4 {
                let mut m = Matrix4x4::new([[2.0; 4]; 4]);
                let mc = m;
                m.m[row][col] = 1.0;
                assert_relative_ne!(m, mc);
                assert_relative_ne!(m, mc);
                assert_relative_eq!(m, mc, epsilon = 1.0);
                assert_relative_eq!(m, mc, epsilon = 0.0, max_relative = 0.5);
            }
        }
    }
}
