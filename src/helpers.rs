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
    ($($vt:ty),+) => {
        $(impl Mini for $vt {
            fn mini(&self, other: $vt) -> $vt {
                self.min(other)
            }
        })*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_float {
    ($($vt:ty),+) => {
        $(impl Maxi for $vt {
            fn maxi(&self, other: $vt) -> $vt {
                self.max(other)
            }
        })*
    }
}
impl_mini_float!(f32, f64);

macro_rules! impl_mini_integer {
    ($($vt:ty),+) => {
        $(impl Mini for $vt {
            fn mini(&self, other: $vt) -> $vt {
                *self.min(&other)
            }
        })*
    }
}
impl_mini_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! impl_maxi_integer {
    ($($vt:ty),+) => {
        $(impl Maxi for $vt {
            fn maxi(&self, other: $vt) -> $vt {
                *self.max(&other)
            }
        })*
    }
}
impl_maxi_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

#[macro_export]
macro_rules! impl_abs_diff_eq {
    ($( $ct:ident<$vt:ty> [ $( $c:ident )+ ] ),+ ) => {
        $(impl approx::AbsDiffEq for $ct<$vt> {
            type Epsilon = Self;

            fn default_epsilon() -> Self::Epsilon {
                Self {
                    $($c: <$vt>::default_epsilon(),)*
                }
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool{
                true $( && self.$c.abs_diff_eq(&other.$c, epsilon.$c) )*
            }
        })*
    };
}

#[macro_export]
macro_rules! impl_vec_vec_op {
    ($ct:ident $vt:ident $rt:ident [ $( $c:ident )+ ] $tr:ident $fun:ident $op:tt) => {
        impl<T> $tr for $ct<T>
        where
            T: ValueType,
        {
            type Output = $rt<T>;

            fn $fun(self, rhs: $vt<T>) -> $rt<T> {
                debug_assert!(!self.has_nans());
                debug_assert!(!rhs.has_nans());

                Self {
                    $($c: self.$c $op rhs.$c,)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_vec_assign_op {
    ($ct:ident $vt:ident[ $( $c:ident )+ ] $tr:ident $fun:ident $op:tt) => {
        impl<T> $tr<$vt<T>> for $ct<T>
        where
            T: ValueType,
        {
            fn $fun(&mut self, rhs: Self) {
                debug_assert!(!self.has_nans());
                debug_assert!(!rhs.has_nans());

                *self = *self $op rhs;
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_scalar_op {
    ($ct:ident [ $( $c:ident )+ ] $tr:ident $fun:ident $op:tt) => {
        impl<T> $tr<T> for $ct<T>
        where
            T: ValueType,
        {
            type Output = Self;

            fn $fun(self, rhs: T) -> Self {
                debug_assert!(!self.has_nans());

                Self {
                    $($c: self.$c $op rhs,)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_scalar_assign_op {
    ($ct:ident [ $( $c:ident )+ ] $tr:ident $fun:ident $op:tt) => {
        impl<T> $tr<T> for $ct<T>
        where
            T: ValueType,
        {
            fn $fun(&mut self, rhs: T) {
                debug_assert!(!self.has_nans());

                *self = *self $op rhs;

                debug_assert!(!self.has_nans());
            }
        }
    }
}

#[macro_export]
macro_rules! impl_vec_index {
    ($( $ct:ident [ $( $ci:expr,$c:ident )+ ] ),+ ) => {
        $(
          impl<T> Index<usize> for $ct<T>
          where
              T: ValueType,
          {
              type Output = T;

              fn index(&self, component: usize) -> &Self::Output {
                  debug_assert!(!self.has_nans());

                  match component {
                      $($ci => &self.$c,)*
                      _ => {
                          panic!("Out of bounds Vec access with component {}", component);
                      }
                  }
              }
          }

          impl<T> IndexMut<usize> for $ct<T>
          where
              T: ValueType,
          {
              fn index_mut(&mut self, component: usize) -> &mut Self::Output {
                  debug_assert!(!self.has_nans());

                  match component {
                      $($ci => &mut self.$c,)*
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
