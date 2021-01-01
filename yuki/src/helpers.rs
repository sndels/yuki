mod tests {
    use crate::common::ValueType;
    #[cfg(test)]
    use approx::{abs_diff_eq, relative_eq};
    use yuki_derive::*;

    // The impl is generic to type and component count so we'll test with
    // a two-component vector and two value types

    #[impl_abs_diff_eq(f32, f64)]
    #[impl_relative_eq(f32, f64)]
    #[derive(PartialEq)]
    struct Vec2<T>
    where
        T: ValueType,
    {
        x: T,
        y: T,
    }

    #[cfg(test)]
    impl<T> Vec2<T>
    where
        T: ValueType,
    {
        pub fn new(x: T, y: T) -> Self {
            Self { x, y }
        }

        pub fn zeros() -> Self {
            Self {
                x: T::zero(),
                y: T::zero(),
            }
        }

        pub fn ones() -> Self {
            Self {
                x: T::one(),
                y: T::one(),
            }
        }
    }

    #[test]
    fn abs_diff_eq() {
        // Basic cases
        assert!(abs_diff_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::zeros()));
        assert!(!abs_diff_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::ones()));
        // Should fail on diff in any coordinate if no epsilon is given
        assert!(!abs_diff_eq!(&Vec2::new(0.0, 1.0), &Vec2::zeros()));
        assert!(!abs_diff_eq!(&Vec2::new(1.0, 0.0), &Vec2::zeros()));
        // Should fail on diff in any coordinate if epsilon doesn't match
        assert!(!abs_diff_eq!(
            &Vec2::new(0.0, 1.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(1.0, 0.0)
        ));
        assert!(!abs_diff_eq!(
            &Vec2::new(1.0, 0.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(0.0, 1.0)
        ));
        // Should succeed with matching epsilon
        assert!(abs_diff_eq!(
            &Vec2::new(1.0, 1.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(1.0, 1.0)
        ));
    }

    #[test]
    fn relative_eq() {
        // Basic cases
        assert!(relative_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::zeros()));
        assert!(!relative_eq!(&Vec2::<f32>::zeros(), &Vec2::<f32>::ones()));
        // Should fail on diff in any coordinate if no epsilon is given
        assert!(!relative_eq!(&Vec2::new(0.0, 1.0), &Vec2::zeros()));
        assert!(!relative_eq!(&Vec2::new(1.0, 0.0), &Vec2::zeros()));
        // Should fail on diff in any coordinate if epsilon doesn't match
        assert!(!relative_eq!(
            &Vec2::new(0.0, 1.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(1.0, 0.0),
            max_relative = Vec2::zeros()
        ));
        assert!(!relative_eq!(
            &Vec2::new(1.0, 0.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(0.0, 1.0),
            max_relative = Vec2::zeros()
        ));
        // Should succeed with matching epsilon
        assert!(relative_eq!(
            &Vec2::new(1.0, 1.0),
            &Vec2::zeros(),
            epsilon = Vec2::new(1.0, 1.0),
            max_relative = Vec2::zeros()
        ));
        // Should fail on diff in any coordinate if epsilon and max_relative don't match
        assert!(!relative_eq!(
            &Vec2::new(0.0, 2.0),
            &Vec2::ones(),
            epsilon = Vec2::zeros(),
            max_relative = Vec2::new(0.5, 0.0)
        ));
        assert!(!relative_eq!(
            &Vec2::new(2.0, 0.0),
            &Vec2::ones(),
            epsilon = Vec2::zeros(),
            max_relative = Vec2::new(0.0, 0.5)
        ));
        // Should succeed with matching max_relative diff
        assert!(relative_eq!(
            &Vec2::new(2.0, 2.0),
            &Vec2::ones(),
            epsilon = Vec2::zeros(),
            max_relative = Vec2::new(0.5, 0.5)
        ));
    }

    // Don't test vector impl generators here as they need to be tested per use to
    // catch wrong usage
}
