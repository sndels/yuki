use num::cast::{FromPrimitive, ToPrimitive};
use num::traits::Num;

/// Generic type that can be stored in the lib containers
pub trait ValueType: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy {}

// Impl for all matching types
impl<T> ValueType for T where T: Num + Mini + Maxi + PartialOrd + ToPrimitive + FromPrimitive + Copy {}

/// Helper trait to generalize on types that implement `fn min(self,other)`
pub trait Mini {
    fn mini(&self, other: Self) -> Self;
}

/// Helper trait to generalize on types that implement `fn max(self, other)`
pub trait Maxi {
    fn maxi(&self, other: Self) -> Self;
}

macro_rules! impl_mini_float {
    ( $( $t:ty ),+ ) => {
        $(
            impl Mini for $t {
                fn mini(&self, other: $t) -> $t {
                    self.min(other)
                }
            }
        )*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_float {
    ( $( $t:ty ),+ ) => {
        $(
            impl Maxi for $t {
                fn maxi(&self, other: $t) -> $t {
                    self.max(other)
                }
            }
        )*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_integer {
    ( $( $t:ty ),+ ) => {
        $(
            impl Mini for $t {
                fn mini(&self, other: $t) -> $t {
                    *self.min(&other)
                }
            }
        )*
    }
}
impl_mini_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_maxi_integer {
    ( $( $t:ty ),+ ) => {
        $(
            impl Maxi for $t {
                fn maxi(&self, other: $t) -> $t {
                    *self.max(&other)
                }
            }
        )*
    }
}
impl_maxi_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

#[macro_export]
// Incorrect use results in a compile error as duplicate/missing/extra components
// won't compile
macro_rules! impl_vec_approx_eq {
    ( $( $vec_type:ident<$t:ty>
         [ $( $component:ident )+ ]
       ),+
    ) => {
        $(
            impl approx::AbsDiffEq for $vec_type<$t> {
                type Epsilon = Self;

                fn default_epsilon() -> Self::Epsilon {
                    Self {
                        $($component: <$t>::default_epsilon(),)*
                    }
                }

                fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool{
                    $(
                        <$t>::abs_diff_eq(
                            &self.$component, &other.$component, epsilon.$component
                        )
                    )&&*
                }
            }

            impl approx::RelativeEq for $vec_type<$t> {
                fn default_max_relative() -> Self::Epsilon {
                    Self {
                        $($component: <$t>::default_max_relative(),)*
                    }
                }

                fn relative_eq(&self, other: &Self, epsilon: Self::Epsilon, max_relative: Self::Epsilon) -> bool{
                    $(
                        <$t>::relative_eq(
                            &self.$component,
                            &other.$component,
                            epsilon.$component,
                            max_relative.$component
                        )
                    )&&*
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! impl_vec_vec_op {
    ( $vec_type:ident $rhs_type:ident $return_type:ident
      [ $( $component:ident )+ ]
      $trait_name:ident $trait_fn:ident $per_component_op:tt
    ) => {
        impl<T> $trait_name<$rhs_type<T>> for $vec_type<T>
        where
            T: ValueType,
        {
            type Output = $return_type<T>;

            fn $trait_fn(self, rhs: $rhs_type<T>) -> $return_type<T> {
                debug_assert!(!self.has_nans());
                debug_assert!(!rhs.has_nans());

                $return_type {
                    $($component: self.$component $per_component_op rhs.$component,)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_vec_assign_op {
    ( $vec_type:ident $rhs_type:ident
      [ $( $component:ident )+ ]
      $trait_name:ident $trait_fn:ident $op:tt
    ) => {
        impl<T> $trait_name<$rhs_type<T>> for $vec_type<T>
        where
            T: ValueType,
        {
            fn $trait_fn(&mut self, rhs: $rhs_type<T>) {
                debug_assert!(!self.has_nans());
                debug_assert!(!rhs.has_nans());

                *self = *self $op rhs;
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_scalar_op {
    ( $vec_type:ident
      [ $( $component:ident )+ ]
      $trait_name:ident $trait_fn:ident $per_component_op:tt
    ) => {
        impl<T> $trait_name<T> for $vec_type<T>
        where
            T: ValueType,
        {
            type Output = Self;

            fn $trait_fn(self, rhs: T) -> Self {
                debug_assert!(!self.has_nans());

                // Use new since it should catch NaNs from rhs
                Self::new(
                    $(self.$component $per_component_op rhs),*
                )
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_scalar_assign_op {
    ( $vec_type:ident
      [ $( $component:ident )+ ]
      $trait_name:ident $trait_fn:ident $op:tt
    ) => {
        impl<T> $trait_name<T> for $vec_type<T>
        where
            T: ValueType,
        {
            fn $trait_fn(&mut self, rhs: T) {
                debug_assert!(!self.has_nans());

                *self = *self $op rhs;

                debug_assert!(!self.has_nans());
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_index {
    ( $( $vec_type:ident
         [ $( $component_index:expr,$component:ident )+ ]
       ),+
    ) => {
        $(
            impl<T> std::ops::Index<usize> for $vec_type<T>
            where
                T: ValueType,
            {
                type Output = T;

                fn index(&self, component: usize) -> &Self::Output {
                    debug_assert!(!self.has_nans());

                    match component {
                        $($component_index => &self.$component,)*
                        _ => {
                            panic!("Out of bounds Vec access with component {}", component);
                        }
                    }
                }
            }

            impl<T> std::ops::IndexMut<usize> for $vec_type<T>
            where
                T: ValueType,
            {
                fn index_mut(&mut self, component: usize) -> &mut Self::Output {
                    debug_assert!(!self.has_nans());

                    match component {
                        $($component_index => &mut self.$component,)*
                        _ => {
                            panic!("Out of bounds Vec access with component {}", component);
                        }
                    }
                }
            }
        )*
    }
}

mod tests {
    use crate::helpers::ValueType;
    #[cfg(test)]
    use crate::impl_vec_approx_eq;
    #[cfg(test)]
    use approx::{abs_diff_eq, relative_eq};

    // The impl is generic to type and component count so we'll test with
    // a two-component vector and two value types

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
    impl_vec_approx_eq!(
        Vec2<f32> [x y],
        Vec2<f64> [x y]
    );

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
