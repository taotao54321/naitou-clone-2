//! 利き情報関連。

use crate::bitop;
use crate::shogi::*;

/// 盤面上の各マスの利き数を保持する。一方の陣営のみ。
///
/// 内藤九段将棋秘伝においては、遠隔駒の影の利きも考慮される。
/// 遠隔利きが自駒の上にあり、かつその自駒が玉でなく、遠隔利きと同一方向の利きを持てば、
/// 遠隔利きがさらに 1 歩延長される。具体的には以下のケースで成立する:
///
/// * 前への遠隔利きが自軍の 桂、角、玉以外 の上にある場合。
/// * 斜め前への遠隔利きが自軍の 歩、香、桂、飛車、玉以外 の上にある場合。
/// * 左右または後ろへの遠隔利きが自軍の 飛車、金、全ての成駒 の上にある場合。
/// * 斜め後ろへの遠隔利きが自軍の 銀、角、馬、龍 の上にある場合。
///
/// 定義より、影の利きがあるマスには必ず通常の利きが同時に存在する。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct EffectCountBoard([u8; 81]);

impl EffectCountBoard {
    /// 全マスの利き数が 0 の `EffectCountBoard` を返す。
    pub const fn empty() -> Self {
        Self([0; 81])
    }
}

impl std::ops::Index<Square> for EffectCountBoard {
    type Output = u8;

    fn index(&self, sq: Square) -> &Self::Output {
        &self.0[usize::from(sq)]
    }
}

impl std::ops::IndexMut<Square> for EffectCountBoard {
    fn index_mut(&mut self, sq: Square) -> &mut Self::Output {
        &mut self.0[usize::from(sq)]
    }
}

impl std::fmt::Display for EffectCountBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Write as _;

        // 影の利きを考慮しても、周囲 8 マスと桂 2 枚、香、角 2 枚、飛 2 枚が最多なので
        // 8 + 2 + 1 + 2 + 2 = 15 より、16 進で足りる。
        const CHARS: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
        ];

        for row in Row::iter() {
            for col in Col::iter().rev() {
                let sq = Square::from_col_row(col, row);
                let n = usize::from(self[sq]);

                if n < CHARS.len() {
                    f.write_char(CHARS[n])?;
                } else {
                    // 変な値になっていたらその旨わかるように表示。
                    write!(f, "[{}]", n)?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// 盤面上の各マスについて、両陣営の遠隔利きを保持する。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct RangedEffectBoard([DirectionSetPair; 81]);

impl RangedEffectBoard {
    /// 全マスに一切の遠隔利きがない `RangedEffectBoard` を返す。
    pub const fn empty() -> Self {
        Self([DirectionSetPair::empty(); 81])
    }
}

impl std::ops::Index<Square> for RangedEffectBoard {
    type Output = DirectionSetPair;

    fn index(&self, sq: Square) -> &Self::Output {
        unsafe { self.0.get_unchecked(usize::from(sq)) }
    }
}

impl std::ops::IndexMut<Square> for RangedEffectBoard {
    fn index_mut(&mut self, sq: Square) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(usize::from(sq)) }
    }
}

impl std::fmt::Display for RangedEffectBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Write as _;

        // HUM は数字、COM はアルファベットで表示する。
        #[rustfmt::skip]
        const CHARS: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7',
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
        ];

        for row in Row::iter() {
            for col in Col::iter().rev() {
                let sq = Square::from_col_row(col, row);
                let dsp = self[sq];

                // 両陣営合わせて 4 個まで利きを表示。
                f.write_char('[')?;
                let mut value = dsp.0;
                for _ in 0..4 {
                    if value == 0 {
                        f.write_char(' ')?;
                    } else {
                        let idx = bitop::pop_lsb_u16(&mut value) as usize;
                        f.write_char(CHARS[idx])?;
                    }
                }
                f.write_char(']')?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// 両陣営について `DirectionSet` を束ねたもの。
///
/// 下位 8bit が HUM、上位 8bit が COM。
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct DirectionSetPair(u16);

impl DirectionSetPair {
    /// 両陣営とも空集合である `DirectionSetPair` を返す。
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 両陣営とも全ての方向を含む `DirectionSetPair` を返す。
    pub const fn all() -> Self {
        Self(0xFFFF)
    }

    /// 各陣営の `DirectionSet` を束ねた `DirectionSetPair` を返す。
    pub const fn new(dirs_hum: DirectionSet, dirs_com: DirectionSet) -> Self {
        let inner = (dirs_hum.inner() as u16) | ((dirs_com.inner() as u16) << 8);
        Self(inner)
    }

    /// 一方の陣営の `DirectionSet` のみを含む `DirectionSetPair` を返す。
    pub const fn from_part(side: Side, dirs: DirectionSet) -> Self {
        let inner = (dirs.inner() as u16) << (side.inner() << 3);
        Self(inner)
    }

    /// 指定した駒の遠隔利きの方向を表す `DirectionSetPair` を返す。
    /// 陣営を区別する。
    pub const fn from_piece_ranged(pc: Piece) -> Self {
        const BISHOP_DIRS: DirectionSet = DirectionSet::RU
            .or(DirectionSet::RD)
            .or(DirectionSet::LU)
            .or(DirectionSet::LD);
        const ROOK_DIRS: DirectionSet = DirectionSet::R
            .or(DirectionSet::U)
            .or(DirectionSet::D)
            .or(DirectionSet::L);

        // 遠隔利きを持つのは香、角、飛車、馬、龍のみ。
        // 馬の遠隔利きは角と同じ。龍の遠隔利きは飛車と同じ。
        const TABLE: [DirectionSetPair; 32] = [
            DirectionSetPair::empty(),                         // NO_PIECE
            DirectionSetPair::empty(),                         // H_PAWN
            DirectionSetPair::from_part(HUM, DirectionSet::U), // H_LANCE
            DirectionSetPair::empty(),                         // H_KNIGHT
            DirectionSetPair::empty(),                         // H_SILVER
            DirectionSetPair::from_part(HUM, BISHOP_DIRS),     // H_BISHOP
            DirectionSetPair::from_part(HUM, ROOK_DIRS),       // H_ROOK
            DirectionSetPair::empty(),                         // H_GOLD
            DirectionSetPair::empty(),                         // H_KING
            DirectionSetPair::empty(),                         // H_PRO_PAWN
            DirectionSetPair::empty(),                         // H_PRO_LANCE
            DirectionSetPair::empty(),                         // H_PRO_KNIGHT
            DirectionSetPair::empty(),                         // H_PRO_SILVER
            DirectionSetPair::from_part(HUM, BISHOP_DIRS),     // H_HORSE
            DirectionSetPair::from_part(HUM, ROOK_DIRS),       // H_DRAGON
            DirectionSetPair::empty(),                         // (15)
            DirectionSetPair::empty(),                         // (16)
            DirectionSetPair::empty(),                         // C_PAWN
            DirectionSetPair::from_part(COM, DirectionSet::D), // C_LANCE
            DirectionSetPair::empty(),                         // C_KNIGHT
            DirectionSetPair::empty(),                         // C_SILVER
            DirectionSetPair::from_part(COM, BISHOP_DIRS),     // C_BISHOP
            DirectionSetPair::from_part(COM, ROOK_DIRS),       // C_ROOK
            DirectionSetPair::empty(),                         // C_GOLD
            DirectionSetPair::empty(),                         // C_KING
            DirectionSetPair::empty(),                         // C_PRO_PAWN
            DirectionSetPair::empty(),                         // C_PRO_LANCE
            DirectionSetPair::empty(),                         // C_PRO_KNIGHT
            DirectionSetPair::empty(),                         // C_PRO_SILVER
            DirectionSetPair::from_part(COM, BISHOP_DIRS),     // C_HORSE
            DirectionSetPair::from_part(COM, ROOK_DIRS),       // C_DRAGON
            DirectionSetPair::empty(),                         // (31)
        ];

        TABLE[pc.inner() as usize]
    }

    /// 両陣営とも空集合かどうかを返す。
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// 指定した陣営の `DirectionSet` を返す。
    pub const fn get(self, side: Side) -> DirectionSet {
        let inner = (self.0 >> (side.inner() << 3)) as u8;
        DirectionSet::from_inner(inner)
    }

    /// 陣営問わず含まれる方向を pop する。`self` は空であってはならない。
    ///
    /// 方向そのものと、その方向について陣営を区別したときの `DirectionSetPair` を返す。
    /// 返される方向の順序は未規定。
    ///
    /// (説明が難しいので、本モジュール内のユニットテストも参照)
    pub fn pop(&mut self) -> (Direction, Self) {
        debug_assert!(!self.is_empty());

        let dir_inner = bitop::lsb_u16(self.0) & 7;
        let dir = Direction::from_inner(dir_inner);

        let dsp = *self & Self((1 << dir_inner) | (1 << (dir_inner + 8)));
        *self &= !dsp;

        (dir, dsp)
    }
}

impl std::ops::Not for DirectionSetPair {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl std::ops::BitAnd for DirectionSetPair {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for DirectionSetPair {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr for DirectionSetPair {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DirectionSetPair {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitXor for DirectionSetPair {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for DirectionSetPair {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl std::fmt::Debug for DirectionSetPair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.get(HUM), self.get(COM))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn test_direction_set_pair_basic() {
        const DSP_EMPTY: DirectionSetPair = DirectionSetPair::empty();

        assert_eq!(DSP_EMPTY.get(HUM), DirectionSet::empty());
        assert_eq!(DSP_EMPTY.get(COM), DirectionSet::empty());

        const DIRS_HUM: DirectionSet = DirectionSet::RU.or(DirectionSet::L);
        const DIRS_COM: DirectionSet = DirectionSet::R.or(DirectionSet::D);

        assert_eq!(
            DirectionSetPair::from_part(HUM, DIRS_HUM).get(HUM),
            DIRS_HUM
        );
        assert_eq!(
            DirectionSetPair::from_part(HUM, DIRS_HUM).get(COM),
            DirectionSet::empty()
        );
        assert_eq!(
            DirectionSetPair::from_part(COM, DIRS_COM).get(HUM),
            DirectionSet::empty()
        );
        assert_eq!(
            DirectionSetPair::from_part(COM, DIRS_COM).get(COM),
            DIRS_COM
        );

        const DSP: DirectionSetPair = DirectionSetPair::new(DIRS_HUM, DIRS_COM);

        assert_eq!(DSP.get(HUM), DIRS_HUM);
        assert_eq!(DSP.get(COM), DIRS_COM);
    }

    #[test]
    fn test_direction_set_pair_bitop() {
        let dsp1 = DirectionSetPair::new(
            DirectionSet::RU | DirectionSet::U | DirectionSet::LD,
            DirectionSet::RD | DirectionSet::D | DirectionSet::LU,
        );
        let dsp2 = DirectionSetPair::new(
            DirectionSet::RU | DirectionSet::D | DirectionSet::L,
            DirectionSet::RU | DirectionSet::D | DirectionSet::L,
        );

        assert_eq!(
            !dsp1,
            DirectionSetPair::new(
                DirectionSet::R
                    | DirectionSet::RD
                    | DirectionSet::D
                    | DirectionSet::LU
                    | DirectionSet::L,
                DirectionSet::RU
                    | DirectionSet::R
                    | DirectionSet::U
                    | DirectionSet::L
                    | DirectionSet::LD
            )
        );

        assert_eq!(
            dsp1 & dsp2,
            DirectionSetPair::new(DirectionSet::RU, DirectionSet::D)
        );

        assert_eq!(
            dsp1 | dsp2,
            DirectionSetPair::new(
                DirectionSet::RU
                    | DirectionSet::U
                    | DirectionSet::D
                    | DirectionSet::L
                    | DirectionSet::LD,
                DirectionSet::RU
                    | DirectionSet::RD
                    | DirectionSet::D
                    | DirectionSet::LU
                    | DirectionSet::L
            )
        );

        assert_eq!(
            dsp1 ^ dsp2,
            DirectionSetPair::new(
                DirectionSet::U | DirectionSet::D | DirectionSet::L | DirectionSet::LD,
                DirectionSet::RU | DirectionSet::RD | DirectionSet::LU | DirectionSet::L
            )
        );
    }

    #[test]
    fn test_direction_set_pair_pop() {
        use std::collections::HashSet;

        let mut dsp = DirectionSetPair::new(
            DirectionSet::RU | DirectionSet::L,
            DirectionSet::U | DirectionSet::L,
        );

        // 返される方向の順序は未規定なので、HashSet に入れた結果をテストする。
        let mut set = HashSet::new();
        set.insert(dsp.pop());
        set.insert(dsp.pop());
        set.insert(dsp.pop());

        assert_eq!(
            set,
            HashSet::from_iter([
                (
                    Direction::RU,
                    DirectionSetPair::from_part(HUM, DirectionSet::RU)
                ),
                (
                    Direction::L,
                    DirectionSetPair::new(DirectionSet::L, DirectionSet::L)
                ),
                (
                    Direction::U,
                    DirectionSetPair::from_part(COM, DirectionSet::U)
                ),
            ])
        );

        assert_eq!(dsp, DirectionSetPair::empty());
    }
}
