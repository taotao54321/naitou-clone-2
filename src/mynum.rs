//! 数値演算の独自拡張(`wrapping_add()` の代入版など)。

use num_traits::{WrappingAdd, WrappingSub};

pub trait WrappingAddAssign: WrappingAdd {
    fn wrapping_add_assign(&mut self, rhs: Self) {
        *self = self.wrapping_add(&rhs);
    }
}

impl<T: WrappingAdd> WrappingAddAssign for T {}

pub trait WrappingSubAssign: WrappingSub {
    fn wrapping_sub_assign(&mut self, rhs: Self) {
        *self = self.wrapping_sub(&rhs);
    }
}

impl<T: WrappingSub> WrappingSubAssign for T {}
