use approx::{AbsDiffEq, RelativeEq};
use std::ops::Mul;

use super::{common::FloatValueType, point::Point3, vector::Vec3};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Utilities/Mathematical_Routines.html#Matrix4x4

/// A row-major 4x4 `Matrix4x4`
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Matrix4x4<T>
where
    T: FloatValueType,
{
    /// Raw values in row-major order.
    pub m: [[T; 4]; 4],
}

impl<T> Matrix4x4<T>
where
    T: FloatValueType,
{
    /// Creates a new `Matrix4x4`.
    pub fn new(m: [[T; 4]; 4]) -> Self {
        let ret = Self { m };
        debug_assert!(!ret.has_nans());
        ret
    }

    /// Creates a new identity `Matrix4x4`.
    pub fn identity() -> Self {
        Self {
            m: [
                [T::one(), T::zero(), T::zero(), T::zero()],
                [T::zero(), T::one(), T::zero(), T::zero()],
                [T::zero(), T::zero(), T::one(), T::zero()],
                [T::zero(), T::zero(), T::zero(), T::one()],
            ],
        }
    }

    /// Creates a new `Matrix4x4` filled with zeroes.
    pub fn zeros() -> Self {
        Self {
            m: [
                [T::zero(), T::zero(), T::zero(), T::zero()],
                [T::zero(), T::zero(), T::zero(), T::zero()],
                [T::zero(), T::zero(), T::zero(), T::zero()],
                [T::zero(), T::zero(), T::zero(), T::zero()],
            ],
        }
    }

    /// Checks if this `Matrix4x4` contains NaNs.
    pub fn has_nans(&self) -> bool {
        // NaNs are the rare special case so no need to early out
        self.m
            .iter()
            // Not all T have is_nan() so rely on NaN != NaN
            .flat_map(|row| row.iter().map(|t| t.is_nan()))
            .into_iter()
            .any(|p| p)
    }

    /// Returns the `i`th row of this `Matrix4x4`.
    pub fn row(&self, i: usize) -> [T; 4] {
        self.m[i]
    }

    /// Returns a mutable reference to the `i`th row in this `Matrix4x4`.
    pub fn row_mut(&mut self, i: usize) -> &mut [T; 4] {
        &mut self.m[i]
    }

    /// Returns the `i`th column of this `Matrix4x4`.
    pub fn col(&self, i: usize) -> [T; 4] {
        [self.m[0][i], self.m[1][i], self.m[2][i], self.m[3][i]]
    }

    /// Returns mutable references to the elements in the `i`th column of this `Matrix4x4`.
    pub fn col_mut(&mut self, i: usize) -> [&mut T; 4] {
        // TODO: Check how this performs
        let (first, rest) = self.m.split_at_mut(1);
        let (second, rest) = rest.split_at_mut(1);
        let (third, fourth) = rest.split_at_mut(1);
        [
            &mut first[0][i],
            &mut second[0][i],
            &mut third[0][i],
            &mut fourth[0][i],
        ]
    }

    /// Returns the transpose of this `Matrix4x4`.
    pub fn transposed(&self) -> Self {
        Self {
            m: [
                [self.m[0][0], self.m[1][0], self.m[2][0], self.m[3][0]],
                [self.m[0][1], self.m[1][1], self.m[2][1], self.m[3][1]],
                [self.m[0][2], self.m[1][2], self.m[2][2], self.m[3][2]],
                [self.m[0][3], self.m[1][3], self.m[2][3], self.m[3][3]],
            ],
        }
    }

    /// Returns the inverse of this `Matrix4x4`.
    pub fn inverted(&self) -> Self {
        // Gauss-Jordan elimination with full pivoting
        // TODO: Would Cramer's rule be faster with the same accuracy by using sse/avx?

        // I tried to improve clarity with better naming and concise comments.
        // Note that the original works on non-square matrices so this is slightly
        // simpler

        // All of this leans on the magical fact that we can augment the matrix with an
        // identity matrix and perform elementary row operations on both of them. Note
        // that this implementation makes use of the fact that much of the stored data
        // is known to be zeros or ones so the inverse can be computed with some book-
        // keeping and a single matrix.

        // The main gist of this is a reducing each column in turn to an identity form.
        // This is done through pivoting on both axes before gaussian elimination to
        // maximize numerical stability, but we save on operations by doing stuff in
        // place when possible. We need to keep track of the permutation so we can
        // shuffle the matrix to the correct permutation after all is done.

        let mut mi = self.m;
        // Helpers to keep track of the pivots we've done
        let mut indxc = [0, 0, 0, 0];
        let mut indxr = [0, 0, 0, 0];
        let mut ipiv = [0, 0, 0, 0];

        // Loop over columns, reducing each one in turn
        for col in 0..4 {
            let mut icol = 0;
            let mut irow = 0;
            let mut big = T::zero();

            // Search for a pivot, i.e.
            // the largest value in the matrix that is not already part of a pivot
            for row in 0..4 {
                if ipiv[row] != 1 {
                    for (rcol, &piv) in ipiv.iter().enumerate() {
                        if (piv == 0) && (mi[row][rcol].abs() > big) {
                            big = mi[row][rcol].abs();
                            irow = row;
                            icol = rcol;
                        }
                    }
                }
            }
            // Mark the pivot as used
            ipiv[icol] += 1;

            // We need to swap rows so that the pivot is on correct row
            if irow != icol {
                // This check is unfortunate but we need split_at_mut
                if irow > icol {
                    let (top, bottom) = mi.split_at_mut(irow);
                    std::mem::swap(&mut top[icol], &mut bottom[0]);
                } else {
                    let (top, bottom) = mi.split_at_mut(icol);
                    std::mem::swap(&mut top[irow], &mut bottom[0]);
                }
            }

            // The pivot still might not be on the diagonal, but we don't care yet
            // so we just take note of where it was
            indxr[col] = irow;
            indxc[col] = icol;

            assert!(mi[icol][icol] != T::zero(), "Can't invert, singular matrix");

            // Let's make the diagonal a 1
            let pivinv = T::one() / mi[icol][icol];
            mi[icol][icol] = T::one();
            // And update the corresponding row accordingly
            for l in 0..4 {
                mi[icol][l] *= pivinv;
            }

            // Zero the column on other rows
            for row in 0..4 {
                if row != icol {
                    let factor = mi[row][icol];
                    mi[row][icol] = T::zero();
                    for rcol in 0..4 {
                        mi[row][rcol] -= factor * mi[icol][rcol];
                    }
                }
            }
        }

        // The inverse might still be jumbled since we didn't pivot columns in memory
        // so we'll finish the pivot here
        for col in (0..4).rev() {
            if indxr[col] != indxc[col] {
                // This check is unfortunate but we need split_at_mut
                let (a, b) = {
                    let a = indxr[col];
                    let b = indxc[col];
                    if a < b {
                        (a, b)
                    } else {
                        (b, a)
                    }
                };
                for row in &mut mi {
                    let (front, back) = row.split_at_mut(b);
                    std::mem::swap(&mut front[a], &mut back[0]);
                }
            }
        }
        Matrix4x4::new(mi)
    }

    /// Tries to decompose the matrix into translation, rotation and scaling
    pub fn decompose(&self) -> Result<DecomposedMatrix<T>, String> {
        let m = &self.m;

        let translation = Point3::new(m[0][3], m[1][3], m[2][3]);

        let scale = Vec3::new(
            Vec3::new(m[0][0], m[1][0], m[2][0]).len(),
            Vec3::new(m[0][1], m[1][1], m[2][1]).len(),
            Vec3::new(m[0][2], m[1][2], m[2][2]).len(),
        );

        if scale.x == T::zero() || scale.y == T::zero() || scale.z == T::zero() {
            return Err("Cannot decompose matrix with a zero scale component".into());
        }

        let mr = [
            [m[0][0] / scale.x, m[0][1] / scale.y, m[0][2] / scale.z],
            [m[1][0] / scale.x, m[1][1] / scale.y, m[1][2] / scale.z],
            [m[2][0] / scale.x, m[2][1] / scale.y, m[2][2] / scale.z],
        ];

        // Extracting Euler Angles from a Rotation Matrix
        // Mike Day, Insomniac Games
        let theta_x = mr[1][2].atan2(mr[2][2]);
        let c2 = (mr[0][0] * mr[0][0] + mr[0][1] * mr[0][1]).sqrt();
        let theta_y = (-mr[0][2]).atan2(c2);
        let s1 = theta_x.sin();
        let c1 = theta_x.cos();
        let theta_z = (s1 * mr[2][0] - c1 * mr[1][0]).atan2(c1 * mr[1][1] - s1 * mr[2][1]);

        let rotation = Vec3::new(theta_x, theta_y, theta_z);

        Ok(DecomposedMatrix {
            translation,
            rotation,
            scale,
        })
    }
}

pub struct DecomposedMatrix<T>
where
    T: FloatValueType,
{
    pub translation: Point3<T>,
    pub rotation: Vec3<T>,
    pub scale: Vec3<T>,
}

impl<T> From<Vec<T>> for Matrix4x4<T>
where
    T: FloatValueType,
{
    fn from(m: Vec<T>) -> Self {
        assert!(m.len() == 16);
        Self::new([
            [m[0], m[1], m[2], m[3]],
            [m[4], m[5], m[6], m[7]],
            [m[8], m[9], m[10], m[11]],
            [m[12], m[13], m[14], m[15]],
        ])
    }
}

// By ref is about twice as fast as by value so let's just endure the syntax
impl<'a, 'b, T> Mul<&'b Matrix4x4<T>> for &'a Matrix4x4<T>
where
    T: FloatValueType,
{
    type Output = Matrix4x4<T>;

    fn mul(self, other: &'b Matrix4x4<T>) -> Matrix4x4<T> {
        let mut ret = Matrix4x4::zeros();
        for row in 0..4 {
            for col in 0..4 {
                ret.m[row][col] = self.m[row][0] * other.m[0][col]
                    + self.m[row][1] * other.m[1][col]
                    + self.m[row][2] * other.m[2][col]
                    + self.m[row][3] * other.m[3][col];
            }
        }
        debug_assert!(!ret.has_nans());
        ret
    }
}

impl<T> AbsDiffEq for Matrix4x4<T>
where
    T: FloatValueType + AbsDiffEq + approx::AbsDiffEq<Epsilon = T>,
{
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        for row in 0..4 {
            for col in 0..4 {
                if !self.m[row][col].abs_diff_eq(&other.m[row][col], epsilon) {
                    return false;
                }
            }
        }
        true
    }
}

impl<T> RelativeEq for Matrix4x4<T>
where
    T: FloatValueType + RelativeEq + approx::AbsDiffEq<Epsilon = T>,
{
    fn default_max_relative() -> Self::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        for row in 0..4 {
            for col in 0..4 {
                if !self.m[row][col].relative_eq(&other.m[row][col], epsilon, max_relative) {
                    return false;
                }
            }
        }
        true
    }
}
