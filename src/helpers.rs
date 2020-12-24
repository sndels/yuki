/// Helper trait to generalize on types that implement `fn min(self,other)`
pub trait Mini {
    fn mini(&self, other: Self) -> Self;
}

/// Helper trait to generalize on types that implement `fn max(self, other)`
pub trait Maxi {
    fn maxi(&self, other: Self) -> Self;
}

impl Mini for i32 {
    fn mini(&self, other: i32) -> i32 {
        *self.min(&other)
    }
}
impl Mini for u32 {
    fn mini(&self, other: u32) -> u32 {
        *self.min(&other)
    }
}
impl Mini for f32 {
    fn mini(&self, other: f32) -> f32 {
        self.min(other)
    }
}

impl Maxi for i32 {
    fn maxi(&self, other: i32) -> i32 {
        *self.max(&other)
    }
}
impl Maxi for u32 {
    fn maxi(&self, other: u32) -> u32 {
        *self.max(&other)
    }
}
impl Maxi for f32 {
    fn maxi(&self, other: f32) -> f32 {
        self.max(other)
    }
}
