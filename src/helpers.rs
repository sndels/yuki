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
macro_rules! impl_abs_diff_eq {
    ( $( $vec_type:ident<$t:ty>
         [ $( $component:ident )+ ]
       ),+
    ) => {
        $(impl approx::AbsDiffEq for $vec_type<$t> {
            type Epsilon = Self;

            fn default_epsilon() -> Self::Epsilon {
                Self {
                    $($component: <$t>::default_epsilon(),)*
                }
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool{
                true $( && self.$component.abs_diff_eq(&other.$component, epsilon.$component) )*
            }
        })*
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

                Self {
                    $($component: self.$component $per_component_op rhs,)*
                }
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
            impl<T> Index<usize> for $vec_type<T>
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

            impl<T> IndexMut<usize> for $vec_type<T>
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

    #[test]
    fn relative_eq() {
        use crate::helpers::ValueType;
        use crate::impl_abs_diff_eq;
        use approx::abs_diff_eq;

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
        }
        impl_abs_diff_eq!(
            Vec2<f32> [x y],
            Vec2<f64> [x y]
        );

        let v0: Vec2<f32> = Vec2::zeros();
        assert!(abs_diff_eq!(v0, v0));
        let v1 = Vec2::new(0.1, 0.0);
        assert!(!abs_diff_eq!(v0, v1));
        let v1 = Vec2::new(0.0, 0.1);
        assert!(!abs_diff_eq!(v0, v1));
        let v1 = Vec2::new(0.0, 0.1);
        assert!(!abs_diff_eq!(v0, v1, epsilon = Vec2::new(0.1, 0.0)));
        let v1 = Vec2::new(0.1, 0.0);
        assert!(!abs_diff_eq!(v0, v1, epsilon = Vec2::new(0.0, 0.1)));
        let v1 = Vec2::new(0.1, 0.1);
        assert!(abs_diff_eq!(v0, v1, epsilon = Vec2::new(0.1, 0.1)));

        let v0: Vec2<f64> = Vec2::zeros();
        assert!(abs_diff_eq!(v0, v0));
    }

    // Don't test vector impl generators here as they need to be tested per use to
    // catch wrong usage
}
