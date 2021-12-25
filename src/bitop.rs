#![allow(dead_code)]

use core::arch::x86_64::*;

/// x の最下位の 1 のビット位置を返す。x == 0 のときの挙動は未定義。
pub fn lsb_u8(x: u8) -> u32 {
    debug_assert_ne!(x, 0);

    x.trailing_zeros()
}

/// x の最下位の 1 を 0 に変え、そのビット位置を返す。x == 0 のときの挙動は未定義。
pub fn pop_lsb_u8(x: &mut u8) -> u32 {
    debug_assert_ne!(*x, 0);

    let res = lsb_u8(*x);
    *x = blsr_u8(*x);
    res
}

/// x の最下位の 1 を 0 に変えたものを返す。x == 0 のときの挙動は未定義。
pub fn blsr_u8(x: u8) -> u8 {
    debug_assert_ne!(x, 0);

    x & (x - 1)
}

/// x の最下位の 1 のビット位置を返す。x == 0 のときの挙動は未定義。
pub fn lsb_u16(x: u16) -> u32 {
    debug_assert_ne!(x, 0);

    x.trailing_zeros()
}

/// x の最下位の 1 を 0 に変え、そのビット位置を返す。x == 0 のときの挙動は未定義。
pub fn pop_lsb_u16(x: &mut u16) -> u32 {
    debug_assert_ne!(*x, 0);

    let res = lsb_u16(*x);
    *x = blsr_u16(*x);
    res
}

/// x の最下位の 1 を 0 に変えたものを返す。x == 0 のときの挙動は未定義。
pub fn blsr_u16(x: u16) -> u16 {
    debug_assert_ne!(x, 0);

    x & (x - 1)
}

/// x の最下位の 1 のビット位置を返す。x == 0 のときの挙動は未定義。
pub fn lsb_u32(x: u32) -> u32 {
    debug_assert_ne!(x, 0);

    x.trailing_zeros()
}

/// x の最下位の 1 を 0 に変え、そのビット位置を返す。x == 0 のときの挙動は未定義。
pub fn pop_lsb_u32(x: &mut u32) -> u32 {
    debug_assert_ne!(*x, 0);

    let res = lsb_u32(*x);
    *x = blsr_u32(*x);
    res
}

/// x の最下位の 1 を 0 に変えたものを返す。x == 0 のときの挙動は未定義。
pub fn blsr_u32(x: u32) -> u32 {
    debug_assert_ne!(x, 0);

    unsafe { _blsr_u32(x) }
}

/// x 内の 1 のビット位置を昇順に列挙する。
pub fn iter_ones_u32(mut x: u32) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || (x != 0).then(|| pop_lsb_u32(&mut x)))
}

/// x の最下位の 1 のビット位置を返す。x == 0 のときの挙動は未定義。
pub fn lsb_u64(x: u64) -> u32 {
    debug_assert_ne!(x, 0);

    x.trailing_zeros()
}

/// x の最下位の 1 を 0 に変え、そのビット位置を返す。x == 0 のときの挙動は未定義。
pub fn pop_lsb_u64(x: &mut u64) -> u32 {
    debug_assert_ne!(*x, 0);

    let res = lsb_u64(*x);
    *x = blsr_u64(*x);
    res
}

/// x の最下位の 1 を 0 に変えたものを返す。x == 0 のときの挙動は未定義。
pub fn blsr_u64(x: u64) -> u64 {
    debug_assert_ne!(x, 0);

    unsafe { _blsr_u64(x) }
}

#[cfg(test)]
mod tests {
    use super::*;

    use itertools::assert_equal;

    #[test]
    fn test_iter_ones_u32() {
        assert_equal(iter_ones_u32(0), []);
        assert_equal(iter_ones_u32(1 << 31), [31]);
        assert_equal(iter_ones_u32(0b10100110), [1, 2, 5, 7]);
    }
}
