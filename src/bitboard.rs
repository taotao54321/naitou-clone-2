use core::arch::x86_64::*;

use crate::bbs;
use crate::bitop;
use crate::shogi::*;

/// 縦型 bitboard。
///
/// bit0:１一, bit1:１二, ..., bit62:７九, bit63:(未使用),
/// bit64:８一, bit65:８二, ..., bit81:９九
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Bitboard(__m128i);

impl Bitboard {
    /// 全マスが 0 の bitboard を作る。盤面外は 0 になっている。
    pub fn zero() -> Self {
        let inner = unsafe { _mm_setzero_si128() };
        Self(inner)
    }

    /// 全マスが 1 の bitboard を作る。盤面外は 0 になっている。
    pub fn all() -> Self {
        let inner = unsafe { _mm_set_epi64x(0x3FFFF, 0x7FFFFFFFFFFFFFFF) };
        Self(inner)
    }

    /// 下位 64bit, 上位 64 bit を指定して bitboard を作る。
    pub fn from_parts(lo: u64, hi: u64) -> Self {
        let inner = unsafe { _mm_set_epi64x(hi as i64, lo as i64) };
        Self(inner)
    }

    /// 盤面外が全て 0 になっているかどうかを返す。
    pub fn is_valid(self) -> bool {
        Self::all().andnot(self).is_zero()
    }

    /// 下位 64bit を返す。
    pub fn part0(self) -> u64 {
        let lo = unsafe { _mm_extract_epi64::<0>(self.0) };
        lo as u64
    }

    /// 上位 64bit を返す。
    pub fn part1(self) -> u64 {
        let hi = unsafe { _mm_extract_epi64::<1>(self.0) };
        hi as u64
    }

    /// `i == 0` ならば下位 64bit を、`i == 1` ならば上位 64bit を返す。他の値を渡してはならない。
    pub fn part(self, i: u32) -> u64 {
        debug_assert!(i == 0 || i == 1);

        if i == 0 {
            self.part0()
        } else {
            self.part1()
        }
    }

    /// 下位 64bit に lo を代入する。
    pub fn set_part0(&mut self, lo: u64) {
        self.0 = unsafe { _mm_insert_epi64::<0>(self.0, lo as i64) };
    }

    /// 上位 64bit に hi を代入する。
    pub fn set_part1(&mut self, hi: u64) {
        self.0 = unsafe { _mm_insert_epi64::<1>(self.0, hi as i64) };
    }

    /// 下位 64bit, 上位 64bit を指定して値を代入する。
    pub fn set_parts(&mut self, lo: u64, hi: u64) {
        self.0 = unsafe { _mm_set_epi64x(hi as i64, lo as i64) };
    }

    /// 全ビット(盤面外含む)が 0 かどうかを返す。
    pub fn is_zero(self) -> bool {
        let res = unsafe { _mm_test_all_zeros(self.0, self.0) };
        res != 0
    }

    /// self と other を AND した結果が非 0 かどうかを返す。
    pub fn test(self, other: Self) -> bool {
        let is_zero = unsafe { _mm_test_all_zeros(self.0, other.0) };
        is_zero == 0
    }

    /// 指定したマスに 1 が立っているかどうかを返す。
    pub fn test_square(self, sq: Square) -> bool {
        self.test(Self::from(sq))
    }

    /// (self の NOT) と rhs の AND を返す。
    /// NOT は盤面外に対しても行われることに注意。
    pub fn andnot(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_andnot_si128(self.0, rhs.0) };
        Self(inner)
    }

    /// 下位/上位 64bit を独立に加算した bitboard を返す。
    pub fn add_parts(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_add_epi64(self.0, rhs.0) };
        Self(inner)
    }

    /// 下位/上位 64bit を独立に減算した bitboard を返す。
    pub fn sub_parts(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_sub_epi64(self.0, rhs.0) };
        Self(inner)
    }

    /// 下位/上位 64bit をそれぞれ `N` 回論理左シフトした bitboard を返す。
    pub fn logical_shift_left_parts<const N: i32>(self) -> Self {
        let inner = unsafe { _mm_slli_epi64::<N>(self.0) };
        Self(inner)
    }

    /// 下位/上位 64bit をそれぞれ `N` 回論理右シフトした bitboard を返す。
    pub fn logical_shift_right_parts<const N: i32>(self) -> Self {
        let inner = unsafe { _mm_srli_epi64::<N>(self.0) };
        Self(inner)
    }

    /// 1 のビットの個数を返す。
    pub fn count_ones(self) -> u32 {
        self.part0().count_ones() + self.part1().count_ones()
    }

    /// 最下位の 1 に対応するマスを返す。self は 0 であってはならない。
    pub fn get_least_square(self) -> Square {
        debug_assert!(!self.is_zero());

        let lo = self.part0();
        let i = if lo != 0 {
            bitop::lsb_u64(lo)
        } else {
            let hi = self.part1();
            63 + bitop::lsb_u64(hi)
        };

        Square::from_inner(i as i32)
    }

    /// 最下位の 1 を 0 に変え、そのビット位置に対応するマスを返す。
    /// self は 0 であってはならない。
    ///
    /// ループ内でこの関数を呼んでいる場合、より速い `for_each_square()` が使えないか検討すること。
    pub fn pop_least_square(&mut self) -> Square {
        debug_assert!(!self.is_zero());

        let mut lo = self.part0();
        let i = if lo != 0 {
            let i = bitop::pop_lsb_u64(&mut lo);
            self.set_part0(lo);
            i
        } else {
            let mut hi = self.part1();
            let i = 63 + bitop::pop_lsb_u64(&mut hi);
            self.set_part1(hi);
            i
        };

        Square::from_inner(i as i32)
    }

    /// 1 が立っているマスを昇順に列挙する。
    /// 速度が要求されるところでは代わりに `for_each_square()` を使うなどすること。
    pub fn squares(self) -> impl Iterator<Item = Square> {
        let mut bb = self;
        std::iter::from_fn(move || (!bb.is_zero()).then(|| bb.pop_least_square()))
    }

    /// 1 が立っている全てのマスについて `f` を呼ぶ。
    ///
    /// これは `pop_least_square()` を呼ぶループより速い。
    /// 下位 64bit が 0 かどうかを毎回チェックする必要がないため。
    pub fn for_each_square<F>(self, mut f: F)
    where
        F: FnMut(Square),
    {
        {
            let mut lo = self.part0();
            while lo != 0 {
                let i = bitop::pop_lsb_u64(&mut lo);
                let sq = Square::from_inner(i as i32);
                f(sq);
            }
        }

        {
            let mut hi = self.part1();
            while hi != 0 {
                let i = 63 + bitop::pop_lsb_u64(&mut hi);
                let sq = Square::from_inner(i as i32);
                f(sq);
            }
        }
    }

    /// バイト単位で反転した bitboard を返す。
    pub fn byte_reverse(self) -> Self {
        let idxs = unsafe { _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15) };
        let inner = unsafe { _mm_shuffle_epi8(self.0, idxs) };
        Self(inner)
    }

    /// 2 つの bitboard の下位/上位 64bit をそれぞれ unpack する。
    ///
    /// unpack の逆変換は unpack である。つまり、この操作を 2 回行うと元に戻る。
    ///
    /// ```
    /// # use naitou_clone::Bitboard;
    /// let bb_lo_in = Bitboard::from_parts(1, 2);
    /// let bb_hi_in = Bitboard::from_parts(3, 4);
    ///
    /// let (bb_lo_out, bb_hi_out) = Bitboard::unpack_pair(bb_lo_in, bb_hi_in);
    ///
    /// assert_eq!(bb_lo_out, Bitboard::from_parts(1, 3));
    /// assert_eq!(bb_hi_out, Bitboard::from_parts(2, 4));
    ///
    /// assert_eq!(Bitboard::unpack_pair(bb_lo_out, bb_hi_out), (bb_lo_in, bb_hi_in));
    /// ```
    pub fn unpack_pair(bb_lo: Bitboard, bb_hi: Bitboard) -> (Bitboard, Bitboard) {
        let lo_inner = unsafe { _mm_unpacklo_epi64(bb_lo.0, bb_hi.0) };
        let hi_inner = unsafe { _mm_unpackhi_epi64(bb_lo.0, bb_hi.0) };
        (Self(lo_inner), Self(hi_inner))
    }

    /// unpack された 2 つの bitboard について、それぞれ 128bit 整数とみなしてデクリメントする。
    ///
    /// ```
    /// # use naitou_clone::Bitboard;
    /// let bb_lo_in = Bitboard::from_parts(1, 3);
    /// let bb_hi_in = Bitboard::from_parts(2, 4);
    ///
    /// let (bb_lo_out, bb_hi_out) = Bitboard::decrement_unpacked_pair(bb_lo_in, bb_hi_in);
    ///
    /// assert_eq!(bb_lo_out, Bitboard::from_parts(0, 2));
    /// assert_eq!(bb_hi_out, Bitboard::from_parts(2, 4));
    /// ```
    pub fn decrement_unpacked_pair(bb_lo: Bitboard, bb_hi: Bitboard) -> (Bitboard, Bitboard) {
        // 下位が 0 の場合のみ上位からのボローが生じるので、
        // `hi += if lo == 0 { -1 } else { 0 };` とすればよい。
        let hi_inner =
            unsafe { _mm_add_epi64(bb_hi.0, _mm_cmpeq_epi64(bb_lo.0, _mm_setzero_si128())) };

        // 下位をデクリメント。
        let lo_inner = unsafe { _mm_add_epi64(bb_lo.0, _mm_set1_epi64x(-1)) };

        (Self(lo_inner), Self(hi_inner))
    }

    /// マスが bitboard の下位 64bit に属するかどうかを返す。
    pub fn square_is_part0(sq: Square) -> bool {
        sq <= SQ_79
    }

    /// マスが bitboard の上位 64bit に属するかどうかを返す。
    pub fn square_is_part1(sq: Square) -> bool {
        !Self::square_is_part0(sq)
    }
}

impl Default for Bitboard {
    /// 全ビットが 0 の bitboard を返す。
    fn default() -> Self {
        Self::zero()
    }
}

impl From<Square> for Bitboard {
    /// 与えられたマスのみが 1 の bitboard を返す。
    fn from(sq: Square) -> Self {
        bbs::square(sq)
    }
}

impl Eq for Bitboard {}

impl PartialEq for Bitboard {
    fn eq(&self, other: &Self) -> bool {
        // 値が等しいことは、XOR した結果が 0 であることと同値。
        let neq = unsafe { _mm_xor_si128(self.0, other.0) };
        let res = unsafe { _mm_test_all_zeros(neq, neq) };
        res != 0
    }
}

impl std::ops::Not for Bitboard {
    type Output = Self;

    /// 全マスについて 0 と 1 を反転した bitboard を返す。盤面外は 0 のまま。
    fn not(self) -> Self {
        // 盤面外が変化しないようにする。
        self ^ Self::all()
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_and_si128(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_or_si128(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        let inner = unsafe { _mm_xor_si128(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for row in Row::iter() {
            for col in Col::iter().rev() {
                let sq = Square::from_col_row(col, row);
                let sq_s = if self.test_square(sq) { " *" } else { " ." };
                f.write_str(sq_s)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// 2 つの [`Bitboard`] をまとめたもの。角の利きの計算に使う。
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Bitboard256(__m256i);

impl Bitboard256 {
    /// 全ビットが 0 の `Bitboard256` を返す。
    pub fn zero() -> Self {
        let inner = unsafe { _mm256_setzero_si256() };
        Self(inner)
    }

    /// 同じ bitboard を 2 つに複製し、それを `Bitboard256` として返す。
    pub fn broadcast_bitboard(bb: Bitboard) -> Self {
        let inner = unsafe { _mm256_broadcastsi128_si256(bb.0) };
        Self(inner)
    }

    /// 2 つの bitboard を合わせた `Bitboard256` を返す。
    pub fn from_bitboards(lo: Bitboard, hi: Bitboard) -> Self {
        let lo256 = unsafe { _mm256_castsi128_si256(lo.0) };
        let inner = unsafe { _mm256_inserti128_si256::<1>(lo256, hi.0) };
        Self(inner)
    }

    /// 保持する 2 つの bitboard を OR したものを返す。
    pub fn merge(self) -> Bitboard {
        let lo = unsafe { _mm256_castsi256_si128(self.0) };
        let hi = unsafe { _mm256_extracti128_si256::<1>(self.0) };
        let inner = unsafe { _mm_or_si128(lo, hi) };
        Bitboard(inner)
    }

    /// 保持する 2 つの bitboard をそれぞれバイト単位で反転したものを返す。
    pub fn byte_reverse(self) -> Self {
        #[rustfmt::skip]
        let idxs = unsafe {
            _mm256_set_epi8(
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
            )
        };
        let inner = unsafe { _mm256_shuffle_epi8(self.0, idxs) };
        Self(inner)
    }

    /// 2 つの `Bitboard256` について、下位/上位 bitboard のそれぞれについて
    /// `Bitboard::unpack_pair()` を行う。
    ///
    /// unpack の逆変換は unpack である。つまり、この操作を 2 回行うと元に戻る。
    ///
    /// ```
    /// # use naitou_clone::{Bitboard, Bitboard256};
    /// let bb_lo_in = Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(1, 2),
    ///     Bitboard::from_parts(3, 4)
    /// );
    /// let bb_hi_in = Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(5, 6),
    ///     Bitboard::from_parts(7, 8)
    /// );
    ///
    /// let (bb_lo_out, bb_hi_out) = Bitboard256::unpack_pair(bb_lo_in, bb_hi_in);
    ///
    /// assert_eq!(bb_lo_out, Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(1, 5),
    ///     Bitboard::from_parts(3, 7),
    /// ));
    /// assert_eq!(bb_hi_out, Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(2, 6),
    ///     Bitboard::from_parts(4, 8),
    /// ));
    ///
    /// assert_eq!(Bitboard256::unpack_pair(bb_lo_out, bb_hi_out), (bb_lo_in, bb_hi_in));
    /// ```
    pub fn unpack_pair(bb_lo: Bitboard256, bb_hi: Bitboard256) -> (Bitboard256, Bitboard256) {
        let lo_inner = unsafe { _mm256_unpacklo_epi64(bb_lo.0, bb_hi.0) };
        let hi_inner = unsafe { _mm256_unpackhi_epi64(bb_lo.0, bb_hi.0) };
        (Self(lo_inner), Self(hi_inner))
    }

    /// unpack された 2 つの `Bitboard256` を 4 つの 128bit 整数とみなし、それぞれデクリメントする。
    ///
    /// ```
    /// # use naitou_clone::{Bitboard, Bitboard256};
    ///
    /// let bb_lo_in = Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(1, 5),
    ///     Bitboard::from_parts(3, 7),
    /// );
    /// let bb_hi_in = Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(2, 6),
    ///     Bitboard::from_parts(4, 8),
    /// );
    ///
    /// let (bb_lo_out, bb_hi_out) = Bitboard256::decrement_unpacked_pair(bb_lo_in, bb_hi_in);
    ///
    /// assert_eq!(bb_lo_out, Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(0, 4),
    ///     Bitboard::from_parts(2, 6),
    /// ));
    /// assert_eq!(bb_hi_out, Bitboard256::from_bitboards(
    ///     Bitboard::from_parts(2, 6),
    ///     Bitboard::from_parts(4, 8),
    /// ));
    /// ```
    pub fn decrement_unpacked_pair(
        bb_lo: Bitboard256,
        bb_hi: Bitboard256,
    ) -> (Bitboard256, Bitboard256) {
        // 下位が 0 の場合のみ上位からのボローが生じるので、
        // `hi += if lo == 0 { -1 } else { 0 };` とすればよい。
        let hi_inner = unsafe {
            _mm256_add_epi64(bb_hi.0, _mm256_cmpeq_epi64(bb_lo.0, _mm256_setzero_si256()))
        };

        // 下位をデクリメント。
        let lo_inner = unsafe { _mm256_add_epi64(bb_lo.0, _mm256_set1_epi64x(-1)) };

        (Self(lo_inner), Self(hi_inner))
    }
}

impl Default for Bitboard256 {
    /// 全ビットが 0 の `Bitboard256` を返す。
    fn default() -> Self {
        Self::zero()
    }
}

impl Eq for Bitboard256 {}

impl PartialEq for Bitboard256 {
    fn eq(&self, other: &Self) -> bool {
        // 値が等しいことは、XOR した結果が 0 であることと同値。
        let neq = unsafe { _mm256_xor_si256(self.0, other.0) };
        let res = unsafe { _mm256_testz_si256(neq, neq) };
        res != 0
    }
}

impl std::ops::BitAnd for Bitboard256 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        let inner = unsafe { _mm256_and_si256(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitAndAssign for Bitboard256 {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr for Bitboard256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        let inner = unsafe { _mm256_or_si256(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitOrAssign for Bitboard256 {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitXor for Bitboard256 {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        let inner = unsafe { _mm256_xor_si256(self.0, rhs.0) };
        Self(inner)
    }
}

impl std::ops::BitXorAssign for Bitboard256 {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use itertools::assert_equal;
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne};

    fn bb_from_sqs(sqs: impl IntoIterator<Item = Square>) -> Bitboard {
        sqs.into_iter()
            .map(Bitboard::from)
            .reduce(std::ops::BitOr::bitor)
            .expect("sqs must be nonempty")
    }

    #[test]
    fn test_basic() {
        let mut bb = Bitboard::zero();
        assert_eq!(bb.part0(), 0);
        assert_eq!(bb.part1(), 0);
        assert!(bb.is_valid());
        assert!(bb.is_zero());

        {
            // これは盤面外に 1 を含むため valid でないことに注意。
            let lo = 0x0123456789ABCDEF;
            let hi = 0xDEADBEEFABADCAFE;
            bb.set_parts(lo, hi);
            assert!(!bb.is_valid());
            assert!(!bb.is_zero());
            assert_eq!(bb, Bitboard::from_parts(lo, hi));
            assert_eq!(bb.part0(), lo);
            assert_eq!(bb.part1(), hi);
            assert_eq!(bb.part(0), bb.part0());
            assert_eq!(bb.part(1), bb.part1());

            let mut bb2 = Bitboard::zero();
            bb2.set_part0(lo);
            bb2.set_part1(hi);
            assert_eq!(bb, bb2);
        }
    }

    #[test]
    fn test_from_square() {
        for sq in Square::iter() {
            let bb = Bitboard::from(sq);
            assert!(bb.is_valid());
            assert!(bb.test_square(sq));
        }
    }

    #[test]
    fn test_bitop() {
        assert_eq!(!bb_from_sqs(Square::iter()), Bitboard::zero());

        let bb1 = bb_from_sqs([SQ_11, SQ_45, SQ_79, SQ_81, SQ_99]);
        let bb2 = bb_from_sqs([SQ_19, SQ_45, SQ_72, SQ_88, SQ_99]);

        assert_eq!(bb1 & bb2, bb_from_sqs([SQ_45, SQ_99]));

        assert_eq!(
            bb1 | bb2,
            bb_from_sqs([SQ_11, SQ_19, SQ_45, SQ_72, SQ_79, SQ_81, SQ_88, SQ_99])
        );

        assert_eq!(
            bb1 ^ bb2,
            bb_from_sqs([SQ_11, SQ_19, SQ_72, SQ_79, SQ_81, SQ_88])
        );
    }

    #[test]
    fn test_add() {
        assert_eq!(
            Bitboard::from_parts(!0, !0).add_parts(Bitboard::from_parts(3, 4)),
            Bitboard::from_parts(2, 3)
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            Bitboard::from_parts(2, 3).sub_parts(Bitboard::from_parts(3, 4)),
            Bitboard::from_parts(!0, !0)
        );
    }

    #[test]
    fn test_shift() {
        let bb = Bitboard::from_parts(0b10110, 0b11001);

        assert_eq!(
            bb.logical_shift_left_parts::<1>(),
            Bitboard::from_parts(0b101100, 0b110010)
        );

        assert_eq!(
            bb.logical_shift_right_parts::<1>(),
            Bitboard::from_parts(0b1011, 0b1100)
        );
    }

    #[test]
    fn test_get_least_square() {
        {
            let bb = bb_from_sqs([SQ_25, SQ_36, SQ_65, SQ_79, SQ_81, SQ_99]);
            assert_eq!(bb.get_least_square(), SQ_25);
        }
        {
            let bb = bb_from_sqs([SQ_84, SQ_93, SQ_99]);
            assert_eq!(bb.get_least_square(), SQ_84);
        }
    }

    #[test]
    fn test_pop_least_square() {
        let mut bb = bb_from_sqs([SQ_11, SQ_39, SQ_79, SQ_81, SQ_94, SQ_99]);

        assert_eq!(bb.pop_least_square(), SQ_11);
        assert_eq!(bb.pop_least_square(), SQ_39);
        assert_eq!(bb.pop_least_square(), SQ_79);
        assert_eq!(bb.pop_least_square(), SQ_81);
        assert_eq!(bb.pop_least_square(), SQ_94);
        assert_eq!(bb.pop_least_square(), SQ_99);

        assert_eq!(bb, Bitboard::zero());
    }

    #[test]
    fn test_squares() {
        let sqs = [SQ_11, SQ_39, SQ_79, SQ_81, SQ_94, SQ_99];

        let bb = bb_from_sqs(sqs);

        assert_equal(bb.squares(), sqs);
    }

    #[test]
    fn test_for_each_square() {
        let sqs_orig = [SQ_11, SQ_39, SQ_79, SQ_81, SQ_94, SQ_99];

        let bb = bb_from_sqs(sqs_orig);
        let mut sqs = vec![];
        bb.for_each_square(|sq| sqs.push(sq));

        assert_eq!(sqs, sqs_orig);
    }

    #[test]
    fn test_byte_reverse() {
        let bb = Bitboard::from_parts(0x0123456789ABCDEF, 0xFEDCBA9876543210);

        assert_eq!(
            bb.byte_reverse(),
            Bitboard::from_parts(0x1032547698BADCFE, 0xEFCDAB8967452301)
        );
    }

    #[test]
    fn test_basic_256() {
        let bb = Bitboard::from_parts(0x0123456789ABCDEF, 0xDEADBEEFABADCAFE);

        assert_eq!(
            Bitboard256::broadcast_bitboard(bb),
            Bitboard256::from_bitboards(bb, bb)
        );
    }

    #[test]
    fn test_bitop_256() {
        let bb1 =
            Bitboard256::from_bitboards(bb_from_sqs([SQ_11, SQ_99]), bb_from_sqs([SQ_45, SQ_67]));
        let bb2 =
            Bitboard256::from_bitboards(bb_from_sqs([SQ_11, SQ_74]), bb_from_sqs([SQ_67, SQ_88]));

        assert_eq!(
            bb1 & bb2,
            Bitboard256::from_bitboards(bb_from_sqs([SQ_11]), bb_from_sqs([SQ_67]))
        );

        assert_eq!(
            bb1 | bb2,
            Bitboard256::from_bitboards(
                bb_from_sqs([SQ_11, SQ_74, SQ_99]),
                bb_from_sqs([SQ_45, SQ_67, SQ_88])
            )
        );

        assert_eq!(
            bb1 ^ bb2,
            Bitboard256::from_bitboards(bb_from_sqs([SQ_74, SQ_99]), bb_from_sqs([SQ_45, SQ_88]))
        );
    }

    #[test]
    fn test_merge_256() {
        let bb = Bitboard256::from_bitboards(
            bb_from_sqs([SQ_11, SQ_85]),
            bb_from_sqs([SQ_34, SQ_85, SQ_99]),
        );

        assert_eq!(bb.merge(), bb_from_sqs([SQ_11, SQ_34, SQ_85, SQ_99]));
    }

    #[test]
    fn test_byte_reverse_256() {
        let bb = Bitboard256::from_bitboards(
            Bitboard::from_parts(0x0123456789ABCDEF, 0xFEDCBA9876543210),
            Bitboard::from_parts(0xDEADBEEFABADCAFE, 0xFEEDFACECAFEBEEF),
        );

        assert_eq!(
            bb.byte_reverse(),
            Bitboard256::from_bitboards(
                Bitboard::from_parts(0x1032547698BADCFE, 0xEFCDAB8967452301),
                Bitboard::from_parts(0xEFBEFECACEFAEDFE, 0xFECAADABEFBEADDE)
            )
        );
    }
}
