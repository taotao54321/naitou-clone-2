//! 将棋の基本要素たち。
//!
//! 駒などは enum ではなく、いわゆる newtype で表現する。
//! たとえば、成駒を容易に求められるように駒の内部値を割り当てたりすることがあるが、
//! enum だと内部値から enum への変換が面倒だし、諸々の最適化がうまくかかるかどうか怪しいため。
//!
//! 筋、段、マスなどの差分は普通のプリミティブ型で扱う。
//! ここまで newtype を導入しても、コード量が増える割にあまり嬉しさがないと思う。
//!
//! 筋、段、マスの内部値は以下のように割り当てている(内藤九段将棋秘伝とは異なる):
//!
//! * 筋は１筋, ２筋, ..., ９筋の順。
//! * 段は一段目, 二段目, ..., 九段目の順。
//! * マスは１一, １二, ..., ９九の順。

use std::iter::FusedIterator;

use crate::bitop;
use crate::myarray::*;

/// 陣営。
///
/// 先手/後手ではなく、HUM/COM という分類にする。
/// 内藤九段将棋秘伝は常に HUM 側を手前として扱うので、この方がわかりやすい。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Side(u32);

pub const HUM: Side = Side(0);
pub const COM: Side = Side(1);

impl Side {
    /// 有効値かどうかを返す。
    pub const fn is_valid(self) -> bool {
        self.0 == HUM.0 || self.0 == COM.0
    }

    /// 敵陣営を返す。
    pub const fn inv(self) -> Side {
        Self(self.0 ^ 1)
    }

    /// 陣営を昇順に列挙する。(`HUM`、`COM` の順)
    pub fn iter(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        [HUM, COM].into_iter()
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> u32 {
        self.0
    }
}

impl From<Side> for u32 {
    fn from(side: Side) -> Self {
        side.0
    }
}

impl From<Side> for usize {
    fn from(side: Side) -> Self {
        debug_assert!(side.is_valid());

        side.0 as Self
    }
}

impl std::fmt::Debug for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            HUM => write!(f, "HUM"),
            COM => write!(f, "COM"),
            _ => write!(f, "Side({})", self.0),
        }
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            HUM => write!(f, "HUM"),
            COM => write!(f, "COM"),
            side => write!(f, "無効な陣営({})", side.0),
        }
    }
}

/// 盤面の筋。たとえば `COL_3` は３筋。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Col(i32);

pub const COL_1: Col = Col(0);
pub const COL_2: Col = Col(1);
pub const COL_3: Col = Col(2);
pub const COL_4: Col = Col(3);
pub const COL_5: Col = Col(4);
pub const COL_6: Col = Col(5);
pub const COL_7: Col = Col(6);
pub const COL_8: Col = Col(7);
pub const COL_9: Col = Col(8);

impl Col {
    /// 内部値を指定して筋を作る。盤面外の値を渡してはならない。
    pub const fn from_inner(inner: i32) -> Self {
        let this = Self(inner);
        debug_assert!(this.is_on_board());

        this
    }

    /// 筋が盤面内かどうかを返す。
    pub const fn is_on_board(self) -> bool {
        COL_1.0 <= self.0 && self.0 <= COL_9.0
    }

    /// 全ての筋を昇順に列挙する。(`COL_1`, `COL_2`, ..., `COL_9` の順)
    pub fn iter(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        // ExactSizeIterator にするため、配列をベタ書きする。
        [
            COL_1, COL_2, COL_3, COL_4, COL_5, COL_6, COL_7, COL_8, COL_9,
        ]
        .into_iter()
    }

    /// 指定した範囲の筋を昇順に列挙する。
    pub fn iter_range(
        min: Self,
        max: Self,
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + FusedIterator {
        (min.0..=max.0).map(Self)
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> i32 {
        self.0
    }
}

impl std::ops::Sub<Self> for Col {
    type Output = i32;

    fn sub(self, rhs: Self) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::Add<i32> for Col {
    type Output = Col;

    fn add(self, rhs: i32) -> Col {
        Col(self.0 + rhs)
    }
}

impl std::ops::Add<Col> for i32 {
    type Output = Col;

    fn add(self, rhs: Col) -> Col {
        Col(self + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for Col {
    fn add_assign(&mut self, rhs: i32) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub<i32> for Col {
    type Output = Col;

    fn sub(self, rhs: i32) -> Col {
        Col(self.0 - rhs)
    }
}

impl std::ops::SubAssign<i32> for Col {
    fn sub_assign(&mut self, rhs: i32) {
        *self = *self - rhs;
    }
}

impl From<Col> for i32 {
    fn from(col: Col) -> Self {
        col.0
    }
}

impl From<Col> for u32 {
    fn from(col: Col) -> Self {
        debug_assert!(col.is_on_board());

        col.0 as Self
    }
}

impl From<Col> for usize {
    fn from(col: Col) -> Self {
        debug_assert!(col.is_on_board());

        col.0 as Self
    }
}

impl std::fmt::Debug for Col {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            COL_1 => write!(f, "COL_1"),
            COL_2 => write!(f, "COL_2"),
            COL_3 => write!(f, "COL_3"),
            COL_4 => write!(f, "COL_4"),
            COL_5 => write!(f, "COL_5"),
            COL_6 => write!(f, "COL_6"),
            COL_7 => write!(f, "COL_7"),
            COL_8 => write!(f, "COL_8"),
            COL_9 => write!(f, "COL_9"),
            _ => write!(f, "Col({})", self.0),
        }
    }
}

impl std::fmt::Display for Col {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            COL_1 => write!(f, "１"),
            COL_2 => write!(f, "２"),
            COL_3 => write!(f, "３"),
            COL_4 => write!(f, "４"),
            COL_5 => write!(f, "５"),
            COL_6 => write!(f, "６"),
            COL_7 => write!(f, "７"),
            COL_8 => write!(f, "８"),
            COL_9 => write!(f, "９"),
            col => write!(f, "無効な筋({})", col.0),
        }
    }
}

/// 盤面の段。たとえば `ROW_3` は三段目。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Row(i32);

pub const ROW_1: Row = Row(0);
pub const ROW_2: Row = Row(1);
pub const ROW_3: Row = Row(2);
pub const ROW_4: Row = Row(3);
pub const ROW_5: Row = Row(4);
pub const ROW_6: Row = Row(5);
pub const ROW_7: Row = Row(6);
pub const ROW_8: Row = Row(7);
pub const ROW_9: Row = Row(8);

impl Row {
    /// 内部値を指定して段を作る。盤面外の値を渡してはならない。
    pub const fn from_inner(inner: i32) -> Self {
        let this = Self(inner);
        debug_assert!(this.is_on_board());

        this
    }

    /// 段が盤面内かどうかを返す。
    pub const fn is_on_board(self) -> bool {
        ROW_1.0 <= self.0 && self.0 <= ROW_9.0
    }

    /// 段が指定した陣営にとって敵陣かどうかを返す。
    pub const fn is_promotion_zone(self, side: Side) -> bool {
        // 両陣営それぞれ 9bit, 計 18bit のビット列を用いて判定すればよいが、
        // 乗算を避けてシフトのみで済ませるため、両陣営それぞれ 16bit, 計 32bit のビット列を用いる。
        // (やねうら王からのパクリ)

        const MASK_HUM: u32 = (1 << ROW_1.0) | (1 << ROW_2.0) | (1 << ROW_3.0);
        const MASK_COM: u32 = (1 << ROW_7.0) | (1 << ROW_8.0) | (1 << ROW_9.0);
        const MASK: u32 = MASK_HUM | (MASK_COM << 16);

        let idx = (side.0 << 4) + (self.0 as u32);

        (MASK & (1 << idx)) != 0
    }

    /// 全ての段を昇順に列挙する。(`ROW_1`, `ROW_2`, ..., `ROW_9` の順)
    pub fn iter(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        // ExactSizeIterator にするため、配列をベタ書きする。
        [
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
        ]
        .into_iter()
    }

    /// 指定した範囲の段を昇順に列挙する。
    pub fn iter_range(
        min: Self,
        max: Self,
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + FusedIterator {
        (min.0..=max.0).map(Self)
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> i32 {
        self.0
    }
}

impl std::ops::Sub<Self> for Row {
    type Output = i32;

    fn sub(self, rhs: Self) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::Add<i32> for Row {
    type Output = Row;

    fn add(self, rhs: i32) -> Row {
        Row(self.0 + rhs)
    }
}

impl std::ops::Add<Row> for i32 {
    type Output = Row;

    fn add(self, rhs: Row) -> Row {
        Row(self + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for Row {
    fn add_assign(&mut self, rhs: i32) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub<i32> for Row {
    type Output = Row;

    fn sub(self, rhs: i32) -> Row {
        Row(self.0 - rhs)
    }
}

impl std::ops::SubAssign<i32> for Row {
    fn sub_assign(&mut self, rhs: i32) {
        *self = *self - rhs;
    }
}

impl From<Row> for i32 {
    fn from(row: Row) -> Self {
        row.0
    }
}

impl From<Row> for u32 {
    fn from(row: Row) -> Self {
        debug_assert!(row.is_on_board());

        row.0 as Self
    }
}

impl From<Row> for usize {
    fn from(row: Row) -> Self {
        debug_assert!(row.is_on_board());

        row.0 as Self
    }
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ROW_1 => write!(f, "ROW_1"),
            ROW_2 => write!(f, "ROW_2"),
            ROW_3 => write!(f, "ROW_3"),
            ROW_4 => write!(f, "ROW_4"),
            ROW_5 => write!(f, "ROW_5"),
            ROW_6 => write!(f, "ROW_6"),
            ROW_7 => write!(f, "ROW_7"),
            ROW_8 => write!(f, "ROW_8"),
            ROW_9 => write!(f, "ROW_9"),
            _ => write!(f, "Row({})", self.0),
        }
    }
}

impl std::fmt::Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ROW_1 => write!(f, "一"),
            ROW_2 => write!(f, "二"),
            ROW_3 => write!(f, "三"),
            ROW_4 => write!(f, "四"),
            ROW_5 => write!(f, "五"),
            ROW_6 => write!(f, "六"),
            ROW_7 => write!(f, "七"),
            ROW_8 => write!(f, "八"),
            ROW_9 => write!(f, "九"),
            row => write!(f, "無効な段({})", row.0),
        }
    }
}

/// 盤面のマス。たとえば `SQ_45` は４五。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Square(i32);

pub const SQ_11: Square = Square::from_col_row(COL_1, ROW_1);
pub const SQ_12: Square = Square::from_col_row(COL_1, ROW_2);
pub const SQ_13: Square = Square::from_col_row(COL_1, ROW_3);
pub const SQ_14: Square = Square::from_col_row(COL_1, ROW_4);
pub const SQ_15: Square = Square::from_col_row(COL_1, ROW_5);
pub const SQ_16: Square = Square::from_col_row(COL_1, ROW_6);
pub const SQ_17: Square = Square::from_col_row(COL_1, ROW_7);
pub const SQ_18: Square = Square::from_col_row(COL_1, ROW_8);
pub const SQ_19: Square = Square::from_col_row(COL_1, ROW_9);
pub const SQ_21: Square = Square::from_col_row(COL_2, ROW_1);
pub const SQ_22: Square = Square::from_col_row(COL_2, ROW_2);
pub const SQ_23: Square = Square::from_col_row(COL_2, ROW_3);
pub const SQ_24: Square = Square::from_col_row(COL_2, ROW_4);
pub const SQ_25: Square = Square::from_col_row(COL_2, ROW_5);
pub const SQ_26: Square = Square::from_col_row(COL_2, ROW_6);
pub const SQ_27: Square = Square::from_col_row(COL_2, ROW_7);
pub const SQ_28: Square = Square::from_col_row(COL_2, ROW_8);
pub const SQ_29: Square = Square::from_col_row(COL_2, ROW_9);
pub const SQ_31: Square = Square::from_col_row(COL_3, ROW_1);
pub const SQ_32: Square = Square::from_col_row(COL_3, ROW_2);
pub const SQ_33: Square = Square::from_col_row(COL_3, ROW_3);
pub const SQ_34: Square = Square::from_col_row(COL_3, ROW_4);
pub const SQ_35: Square = Square::from_col_row(COL_3, ROW_5);
pub const SQ_36: Square = Square::from_col_row(COL_3, ROW_6);
pub const SQ_37: Square = Square::from_col_row(COL_3, ROW_7);
pub const SQ_38: Square = Square::from_col_row(COL_3, ROW_8);
pub const SQ_39: Square = Square::from_col_row(COL_3, ROW_9);
pub const SQ_41: Square = Square::from_col_row(COL_4, ROW_1);
pub const SQ_42: Square = Square::from_col_row(COL_4, ROW_2);
pub const SQ_43: Square = Square::from_col_row(COL_4, ROW_3);
pub const SQ_44: Square = Square::from_col_row(COL_4, ROW_4);
pub const SQ_45: Square = Square::from_col_row(COL_4, ROW_5);
pub const SQ_46: Square = Square::from_col_row(COL_4, ROW_6);
pub const SQ_47: Square = Square::from_col_row(COL_4, ROW_7);
pub const SQ_48: Square = Square::from_col_row(COL_4, ROW_8);
pub const SQ_49: Square = Square::from_col_row(COL_4, ROW_9);
pub const SQ_51: Square = Square::from_col_row(COL_5, ROW_1);
pub const SQ_52: Square = Square::from_col_row(COL_5, ROW_2);
pub const SQ_53: Square = Square::from_col_row(COL_5, ROW_3);
pub const SQ_54: Square = Square::from_col_row(COL_5, ROW_4);
pub const SQ_55: Square = Square::from_col_row(COL_5, ROW_5);
pub const SQ_56: Square = Square::from_col_row(COL_5, ROW_6);
pub const SQ_57: Square = Square::from_col_row(COL_5, ROW_7);
pub const SQ_58: Square = Square::from_col_row(COL_5, ROW_8);
pub const SQ_59: Square = Square::from_col_row(COL_5, ROW_9);
pub const SQ_61: Square = Square::from_col_row(COL_6, ROW_1);
pub const SQ_62: Square = Square::from_col_row(COL_6, ROW_2);
pub const SQ_63: Square = Square::from_col_row(COL_6, ROW_3);
pub const SQ_64: Square = Square::from_col_row(COL_6, ROW_4);
pub const SQ_65: Square = Square::from_col_row(COL_6, ROW_5);
pub const SQ_66: Square = Square::from_col_row(COL_6, ROW_6);
pub const SQ_67: Square = Square::from_col_row(COL_6, ROW_7);
pub const SQ_68: Square = Square::from_col_row(COL_6, ROW_8);
pub const SQ_69: Square = Square::from_col_row(COL_6, ROW_9);
pub const SQ_71: Square = Square::from_col_row(COL_7, ROW_1);
pub const SQ_72: Square = Square::from_col_row(COL_7, ROW_2);
pub const SQ_73: Square = Square::from_col_row(COL_7, ROW_3);
pub const SQ_74: Square = Square::from_col_row(COL_7, ROW_4);
pub const SQ_75: Square = Square::from_col_row(COL_7, ROW_5);
pub const SQ_76: Square = Square::from_col_row(COL_7, ROW_6);
pub const SQ_77: Square = Square::from_col_row(COL_7, ROW_7);
pub const SQ_78: Square = Square::from_col_row(COL_7, ROW_8);
pub const SQ_79: Square = Square::from_col_row(COL_7, ROW_9);
pub const SQ_81: Square = Square::from_col_row(COL_8, ROW_1);
pub const SQ_82: Square = Square::from_col_row(COL_8, ROW_2);
pub const SQ_83: Square = Square::from_col_row(COL_8, ROW_3);
pub const SQ_84: Square = Square::from_col_row(COL_8, ROW_4);
pub const SQ_85: Square = Square::from_col_row(COL_8, ROW_5);
pub const SQ_86: Square = Square::from_col_row(COL_8, ROW_6);
pub const SQ_87: Square = Square::from_col_row(COL_8, ROW_7);
pub const SQ_88: Square = Square::from_col_row(COL_8, ROW_8);
pub const SQ_89: Square = Square::from_col_row(COL_8, ROW_9);
pub const SQ_91: Square = Square::from_col_row(COL_9, ROW_1);
pub const SQ_92: Square = Square::from_col_row(COL_9, ROW_2);
pub const SQ_93: Square = Square::from_col_row(COL_9, ROW_3);
pub const SQ_94: Square = Square::from_col_row(COL_9, ROW_4);
pub const SQ_95: Square = Square::from_col_row(COL_9, ROW_5);
pub const SQ_96: Square = Square::from_col_row(COL_9, ROW_6);
pub const SQ_97: Square = Square::from_col_row(COL_9, ROW_7);
pub const SQ_98: Square = Square::from_col_row(COL_9, ROW_8);
pub const SQ_99: Square = Square::from_col_row(COL_9, ROW_9);

impl Square {
    pub const DIR_R: i32 = -9;
    pub const DIR_U: i32 = -1;
    pub const DIR_D: i32 = -Self::DIR_U;
    pub const DIR_L: i32 = -Self::DIR_R;

    pub const DIR_RU: i32 = Self::DIR_R + Self::DIR_U;
    pub const DIR_RD: i32 = Self::DIR_R + Self::DIR_D;
    pub const DIR_LU: i32 = Self::DIR_L + Self::DIR_U;
    pub const DIR_LD: i32 = Self::DIR_L + Self::DIR_D;
    pub const DIR_RUU: i32 = Self::DIR_RU + Self::DIR_U;
    pub const DIR_RDD: i32 = Self::DIR_RD + Self::DIR_D;
    pub const DIR_LUU: i32 = Self::DIR_LU + Self::DIR_U;
    pub const DIR_LDD: i32 = Self::DIR_LD + Self::DIR_D;

    /// 内部値を指定してマスを作る。盤面外の値を渡してはならない。
    pub const fn from_inner(inner: i32) -> Self {
        let this = Self(inner);
        debug_assert!(this.is_on_board());

        this
    }

    /// 筋と段からマスを作る。
    pub const fn from_col_row(col: Col, row: Row) -> Self {
        Self(9 * col.0 + row.0)
    }

    /// マスが盤面内かどうかを返す。
    pub const fn is_on_board(self) -> bool {
        SQ_11.0 <= self.0 && self.0 <= SQ_99.0
    }

    /// マスの属する筋を返す。
    pub const fn col(self) -> Col {
        #[rustfmt::skip]
        const TABLE: [Col; 81] = [
            COL_1, COL_1, COL_1, COL_1, COL_1, COL_1, COL_1, COL_1, COL_1,
            COL_2, COL_2, COL_2, COL_2, COL_2, COL_2, COL_2, COL_2, COL_2,
            COL_3, COL_3, COL_3, COL_3, COL_3, COL_3, COL_3, COL_3, COL_3,
            COL_4, COL_4, COL_4, COL_4, COL_4, COL_4, COL_4, COL_4, COL_4,
            COL_5, COL_5, COL_5, COL_5, COL_5, COL_5, COL_5, COL_5, COL_5,
            COL_6, COL_6, COL_6, COL_6, COL_6, COL_6, COL_6, COL_6, COL_6,
            COL_7, COL_7, COL_7, COL_7, COL_7, COL_7, COL_7, COL_7, COL_7,
            COL_8, COL_8, COL_8, COL_8, COL_8, COL_8, COL_8, COL_8, COL_8,
            COL_9, COL_9, COL_9, COL_9, COL_9, COL_9, COL_9, COL_9, COL_9,
        ];

        debug_assert!(self.is_on_board());

        TABLE[self.0 as usize]
    }

    /// マスの属する段を返す。
    pub const fn row(self) -> Row {
        #[rustfmt::skip]
        const TABLE: [Row; 81] = [
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
            ROW_1, ROW_2, ROW_3, ROW_4, ROW_5, ROW_6, ROW_7, ROW_8, ROW_9,
        ];

        debug_assert!(self.is_on_board());

        TABLE[self.0 as usize]
    }

    /// 2 つのマスの間のチェス盤距離を返す。`self`, `other` は盤面上のマスでなければならない。
    pub const fn distance(self, other: Self) -> u8 {
        debug_assert!(self.is_on_board());
        debug_assert!(other.is_on_board());

        const TABLE: [[u8; 81]; 81] = {
            let mut res = [[0; 81]; 81];

            let mut sq1_i = 0;
            while sq1_i < 81 {
                let sq1 = Square(sq1_i);

                let mut sq2_i = 0;
                while sq2_i < 81 {
                    let sq2 = Square(sq2_i);

                    let dx_abs = (sq1.col().0 - sq2.col().0).abs() as u8;
                    let dy_abs = (sq1.row().0 - sq2.row().0).abs() as u8;
                    res[sq1.0 as usize][sq2.0 as usize] =
                        if dx_abs < dy_abs { dy_abs } else { dx_abs };

                    sq2_i += 1;
                }

                sq1_i += 1;
            }

            res
        };

        TABLE[self.0 as usize][other.0 as usize]
    }

    /// マスが指定した陣営にとって敵陣かどうかを返す。
    pub const fn is_promotion_zone(self, side: Side) -> bool {
        self.row().is_promotion_zone(side)
    }

    /// 全マスを昇順に列挙する。(`SQ_11`, `SQ_12`, ..., `SQ_99` の順)
    pub fn iter(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        // ExactSizeIterator にするため、配列をベタ書きする。
        #[rustfmt::skip]
        const SQS: [Square; 81] = [
            SQ_11, SQ_12, SQ_13, SQ_14, SQ_15, SQ_16, SQ_17, SQ_18, SQ_19,
            SQ_21, SQ_22, SQ_23, SQ_24, SQ_25, SQ_26, SQ_27, SQ_28, SQ_29,
            SQ_31, SQ_32, SQ_33, SQ_34, SQ_35, SQ_36, SQ_37, SQ_38, SQ_39,
            SQ_41, SQ_42, SQ_43, SQ_44, SQ_45, SQ_46, SQ_47, SQ_48, SQ_49,
            SQ_51, SQ_52, SQ_53, SQ_54, SQ_55, SQ_56, SQ_57, SQ_58, SQ_59,
            SQ_61, SQ_62, SQ_63, SQ_64, SQ_65, SQ_66, SQ_67, SQ_68, SQ_69,
            SQ_71, SQ_72, SQ_73, SQ_74, SQ_75, SQ_76, SQ_77, SQ_78, SQ_79,
            SQ_81, SQ_82, SQ_83, SQ_84, SQ_85, SQ_86, SQ_87, SQ_88, SQ_89,
            SQ_91, SQ_92, SQ_93, SQ_94, SQ_95, SQ_96, SQ_97, SQ_98, SQ_99,
        ];

        SQS.into_iter()
    }

    /// 指定した範囲のマスを昇順に列挙する。
    pub fn iter_range(
        min: Self,
        max: Self,
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + FusedIterator {
        (min.0..=max.0).map(Self)
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> i32 {
        self.0
    }
}

impl std::ops::Sub<Self> for Square {
    type Output = i32;

    fn sub(self, rhs: Self) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::Add<i32> for Square {
    type Output = Square;

    fn add(self, rhs: i32) -> Square {
        Square(self.0 + rhs)
    }
}

impl std::ops::Add<Square> for i32 {
    type Output = Square;

    fn add(self, rhs: Square) -> Square {
        Square(self + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for Square {
    fn add_assign(&mut self, rhs: i32) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub<i32> for Square {
    type Output = Square;

    fn sub(self, rhs: i32) -> Square {
        Square(self.0 - rhs)
    }
}

impl std::ops::SubAssign<i32> for Square {
    fn sub_assign(&mut self, rhs: i32) {
        *self = *self - rhs;
    }
}

impl From<Square> for i32 {
    fn from(sq: Square) -> Self {
        sq.0
    }
}

impl From<Square> for u32 {
    fn from(sq: Square) -> Self {
        debug_assert!(sq.is_on_board());

        sq.0 as Self
    }
}

impl From<Square> for usize {
    fn from(sq: Square) -> Self {
        debug_assert!(sq.is_on_board());

        sq.0 as Self
    }
}

impl std::fmt::Debug for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SQ_11 => write!(f, "SQ_11"),
            SQ_12 => write!(f, "SQ_12"),
            SQ_13 => write!(f, "SQ_13"),
            SQ_14 => write!(f, "SQ_14"),
            SQ_15 => write!(f, "SQ_15"),
            SQ_16 => write!(f, "SQ_16"),
            SQ_17 => write!(f, "SQ_17"),
            SQ_18 => write!(f, "SQ_18"),
            SQ_19 => write!(f, "SQ_19"),
            SQ_21 => write!(f, "SQ_21"),
            SQ_22 => write!(f, "SQ_22"),
            SQ_23 => write!(f, "SQ_23"),
            SQ_24 => write!(f, "SQ_24"),
            SQ_25 => write!(f, "SQ_25"),
            SQ_26 => write!(f, "SQ_26"),
            SQ_27 => write!(f, "SQ_27"),
            SQ_28 => write!(f, "SQ_28"),
            SQ_29 => write!(f, "SQ_29"),
            SQ_31 => write!(f, "SQ_31"),
            SQ_32 => write!(f, "SQ_32"),
            SQ_33 => write!(f, "SQ_33"),
            SQ_34 => write!(f, "SQ_34"),
            SQ_35 => write!(f, "SQ_35"),
            SQ_36 => write!(f, "SQ_36"),
            SQ_37 => write!(f, "SQ_37"),
            SQ_38 => write!(f, "SQ_38"),
            SQ_39 => write!(f, "SQ_39"),
            SQ_41 => write!(f, "SQ_41"),
            SQ_42 => write!(f, "SQ_42"),
            SQ_43 => write!(f, "SQ_43"),
            SQ_44 => write!(f, "SQ_44"),
            SQ_45 => write!(f, "SQ_45"),
            SQ_46 => write!(f, "SQ_46"),
            SQ_47 => write!(f, "SQ_47"),
            SQ_48 => write!(f, "SQ_48"),
            SQ_49 => write!(f, "SQ_49"),
            SQ_51 => write!(f, "SQ_51"),
            SQ_52 => write!(f, "SQ_52"),
            SQ_53 => write!(f, "SQ_53"),
            SQ_54 => write!(f, "SQ_54"),
            SQ_55 => write!(f, "SQ_55"),
            SQ_56 => write!(f, "SQ_56"),
            SQ_57 => write!(f, "SQ_57"),
            SQ_58 => write!(f, "SQ_58"),
            SQ_59 => write!(f, "SQ_59"),
            SQ_61 => write!(f, "SQ_61"),
            SQ_62 => write!(f, "SQ_62"),
            SQ_63 => write!(f, "SQ_63"),
            SQ_64 => write!(f, "SQ_64"),
            SQ_65 => write!(f, "SQ_65"),
            SQ_66 => write!(f, "SQ_66"),
            SQ_67 => write!(f, "SQ_67"),
            SQ_68 => write!(f, "SQ_68"),
            SQ_69 => write!(f, "SQ_69"),
            SQ_71 => write!(f, "SQ_71"),
            SQ_72 => write!(f, "SQ_72"),
            SQ_73 => write!(f, "SQ_73"),
            SQ_74 => write!(f, "SQ_74"),
            SQ_75 => write!(f, "SQ_75"),
            SQ_76 => write!(f, "SQ_76"),
            SQ_77 => write!(f, "SQ_77"),
            SQ_78 => write!(f, "SQ_78"),
            SQ_79 => write!(f, "SQ_79"),
            SQ_81 => write!(f, "SQ_81"),
            SQ_82 => write!(f, "SQ_82"),
            SQ_83 => write!(f, "SQ_83"),
            SQ_84 => write!(f, "SQ_84"),
            SQ_85 => write!(f, "SQ_85"),
            SQ_86 => write!(f, "SQ_86"),
            SQ_87 => write!(f, "SQ_87"),
            SQ_88 => write!(f, "SQ_88"),
            SQ_89 => write!(f, "SQ_89"),
            SQ_91 => write!(f, "SQ_91"),
            SQ_92 => write!(f, "SQ_92"),
            SQ_93 => write!(f, "SQ_93"),
            SQ_94 => write!(f, "SQ_94"),
            SQ_95 => write!(f, "SQ_95"),
            SQ_96 => write!(f, "SQ_96"),
            SQ_97 => write!(f, "SQ_97"),
            SQ_98 => write!(f, "SQ_98"),
            SQ_99 => write!(f, "SQ_99"),
            _ => write!(f, "Square({})", self.0),
        }
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_on_board() {
            write!(f, "{}{}", self.col(), self.row())
        } else {
            write!(f, "無効なマス({})", self.0)
        }
    }
}

/// 壁つきのマス表現。(やねうら王参照)
///
/// * bit0-7:   `Square`
/// * bit8:     初期値 1 (`Square` が負になったとき上位ビットに波及するのを防ぐ)
/// * bit9-13:  このマスから右に何マスあるか (負になると bit13 が 1 になる)
/// * bit14-18: このマスから上に何マスあるか (負になると bit18 が 1 になる)
/// * bit19-23: このマスから下に何マスあるか (負になると bit23 が 1 になる)
/// * bit24-28: このマスから左に何マスあるか (負になると bit28 が 1 になる)
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct SquareWithWall(i32);

pub const SQWW_11: SquareWithWall = SquareWithWall::from_square(SQ_11);
pub const SQWW_12: SquareWithWall = SquareWithWall::from_square(SQ_12);
pub const SQWW_13: SquareWithWall = SquareWithWall::from_square(SQ_13);
pub const SQWW_14: SquareWithWall = SquareWithWall::from_square(SQ_14);
pub const SQWW_15: SquareWithWall = SquareWithWall::from_square(SQ_15);
pub const SQWW_16: SquareWithWall = SquareWithWall::from_square(SQ_16);
pub const SQWW_17: SquareWithWall = SquareWithWall::from_square(SQ_17);
pub const SQWW_18: SquareWithWall = SquareWithWall::from_square(SQ_18);
pub const SQWW_19: SquareWithWall = SquareWithWall::from_square(SQ_19);
pub const SQWW_21: SquareWithWall = SquareWithWall::from_square(SQ_21);
pub const SQWW_22: SquareWithWall = SquareWithWall::from_square(SQ_22);
pub const SQWW_23: SquareWithWall = SquareWithWall::from_square(SQ_23);
pub const SQWW_24: SquareWithWall = SquareWithWall::from_square(SQ_24);
pub const SQWW_25: SquareWithWall = SquareWithWall::from_square(SQ_25);
pub const SQWW_26: SquareWithWall = SquareWithWall::from_square(SQ_26);
pub const SQWW_27: SquareWithWall = SquareWithWall::from_square(SQ_27);
pub const SQWW_28: SquareWithWall = SquareWithWall::from_square(SQ_28);
pub const SQWW_29: SquareWithWall = SquareWithWall::from_square(SQ_29);
pub const SQWW_31: SquareWithWall = SquareWithWall::from_square(SQ_31);
pub const SQWW_32: SquareWithWall = SquareWithWall::from_square(SQ_32);
pub const SQWW_33: SquareWithWall = SquareWithWall::from_square(SQ_33);
pub const SQWW_34: SquareWithWall = SquareWithWall::from_square(SQ_34);
pub const SQWW_35: SquareWithWall = SquareWithWall::from_square(SQ_35);
pub const SQWW_36: SquareWithWall = SquareWithWall::from_square(SQ_36);
pub const SQWW_37: SquareWithWall = SquareWithWall::from_square(SQ_37);
pub const SQWW_38: SquareWithWall = SquareWithWall::from_square(SQ_38);
pub const SQWW_39: SquareWithWall = SquareWithWall::from_square(SQ_39);
pub const SQWW_41: SquareWithWall = SquareWithWall::from_square(SQ_41);
pub const SQWW_42: SquareWithWall = SquareWithWall::from_square(SQ_42);
pub const SQWW_43: SquareWithWall = SquareWithWall::from_square(SQ_43);
pub const SQWW_44: SquareWithWall = SquareWithWall::from_square(SQ_44);
pub const SQWW_45: SquareWithWall = SquareWithWall::from_square(SQ_45);
pub const SQWW_46: SquareWithWall = SquareWithWall::from_square(SQ_46);
pub const SQWW_47: SquareWithWall = SquareWithWall::from_square(SQ_47);
pub const SQWW_48: SquareWithWall = SquareWithWall::from_square(SQ_48);
pub const SQWW_49: SquareWithWall = SquareWithWall::from_square(SQ_49);
pub const SQWW_51: SquareWithWall = SquareWithWall::from_square(SQ_51);
pub const SQWW_52: SquareWithWall = SquareWithWall::from_square(SQ_52);
pub const SQWW_53: SquareWithWall = SquareWithWall::from_square(SQ_53);
pub const SQWW_54: SquareWithWall = SquareWithWall::from_square(SQ_54);
pub const SQWW_55: SquareWithWall = SquareWithWall::from_square(SQ_55);
pub const SQWW_56: SquareWithWall = SquareWithWall::from_square(SQ_56);
pub const SQWW_57: SquareWithWall = SquareWithWall::from_square(SQ_57);
pub const SQWW_58: SquareWithWall = SquareWithWall::from_square(SQ_58);
pub const SQWW_59: SquareWithWall = SquareWithWall::from_square(SQ_59);
pub const SQWW_61: SquareWithWall = SquareWithWall::from_square(SQ_61);
pub const SQWW_62: SquareWithWall = SquareWithWall::from_square(SQ_62);
pub const SQWW_63: SquareWithWall = SquareWithWall::from_square(SQ_63);
pub const SQWW_64: SquareWithWall = SquareWithWall::from_square(SQ_64);
pub const SQWW_65: SquareWithWall = SquareWithWall::from_square(SQ_65);
pub const SQWW_66: SquareWithWall = SquareWithWall::from_square(SQ_66);
pub const SQWW_67: SquareWithWall = SquareWithWall::from_square(SQ_67);
pub const SQWW_68: SquareWithWall = SquareWithWall::from_square(SQ_68);
pub const SQWW_69: SquareWithWall = SquareWithWall::from_square(SQ_69);
pub const SQWW_71: SquareWithWall = SquareWithWall::from_square(SQ_71);
pub const SQWW_72: SquareWithWall = SquareWithWall::from_square(SQ_72);
pub const SQWW_73: SquareWithWall = SquareWithWall::from_square(SQ_73);
pub const SQWW_74: SquareWithWall = SquareWithWall::from_square(SQ_74);
pub const SQWW_75: SquareWithWall = SquareWithWall::from_square(SQ_75);
pub const SQWW_76: SquareWithWall = SquareWithWall::from_square(SQ_76);
pub const SQWW_77: SquareWithWall = SquareWithWall::from_square(SQ_77);
pub const SQWW_78: SquareWithWall = SquareWithWall::from_square(SQ_78);
pub const SQWW_79: SquareWithWall = SquareWithWall::from_square(SQ_79);
pub const SQWW_81: SquareWithWall = SquareWithWall::from_square(SQ_81);
pub const SQWW_82: SquareWithWall = SquareWithWall::from_square(SQ_82);
pub const SQWW_83: SquareWithWall = SquareWithWall::from_square(SQ_83);
pub const SQWW_84: SquareWithWall = SquareWithWall::from_square(SQ_84);
pub const SQWW_85: SquareWithWall = SquareWithWall::from_square(SQ_85);
pub const SQWW_86: SquareWithWall = SquareWithWall::from_square(SQ_86);
pub const SQWW_87: SquareWithWall = SquareWithWall::from_square(SQ_87);
pub const SQWW_88: SquareWithWall = SquareWithWall::from_square(SQ_88);
pub const SQWW_89: SquareWithWall = SquareWithWall::from_square(SQ_89);
pub const SQWW_91: SquareWithWall = SquareWithWall::from_square(SQ_91);
pub const SQWW_92: SquareWithWall = SquareWithWall::from_square(SQ_92);
pub const SQWW_93: SquareWithWall = SquareWithWall::from_square(SQ_93);
pub const SQWW_94: SquareWithWall = SquareWithWall::from_square(SQ_94);
pub const SQWW_95: SquareWithWall = SquareWithWall::from_square(SQ_95);
pub const SQWW_96: SquareWithWall = SquareWithWall::from_square(SQ_96);
pub const SQWW_97: SquareWithWall = SquareWithWall::from_square(SQ_97);
pub const SQWW_98: SquareWithWall = SquareWithWall::from_square(SQ_98);
pub const SQWW_99: SquareWithWall = SquareWithWall::from_square(SQ_99);

impl SquareWithWall {
    pub const DIR_R: i32 = Square::DIR_R - (1 << 9) + (1 << 24);
    pub const DIR_U: i32 = Square::DIR_U - (1 << 14) + (1 << 19);
    pub const DIR_D: i32 = -Self::DIR_U;
    pub const DIR_L: i32 = -Self::DIR_R;

    pub const DIR_RU: i32 = Self::DIR_R + Self::DIR_U;
    pub const DIR_RD: i32 = Self::DIR_R + Self::DIR_D;
    pub const DIR_LU: i32 = Self::DIR_L + Self::DIR_U;
    pub const DIR_LD: i32 = Self::DIR_L + Self::DIR_D;
    pub const DIR_RUU: i32 = Self::DIR_RU + Self::DIR_U;
    pub const DIR_RDD: i32 = Self::DIR_RD + Self::DIR_D;
    pub const DIR_LUU: i32 = Self::DIR_LU + Self::DIR_U;
    pub const DIR_LDD: i32 = Self::DIR_LD + Self::DIR_D;

    const fn from_square(sq: Square) -> Self {
        const TABLE: [SquareWithWall; 81] = {
            // １一の地点からは右に 0 マス、上に 0 マス、下に 8 マス、左に 8 マスある。
            #[allow(clippy::identity_op)]
            const INNER_11: i32 = SQ_11.0 | (1 << 8) | (0 << 9) | (0 << 14) | (8 << 19) | (8 << 24);

            let mut res = [SquareWithWall(0); 81];

            let mut sq_inner = 0;
            while sq_inner < 81 {
                let sq = Square(sq_inner);
                let c = sq.col().0;
                let r = sq.row().0;
                res[sq_inner as usize] = SquareWithWall(
                    INNER_11 + c * SquareWithWall::DIR_L + r * SquareWithWall::DIR_D,
                );
                sq_inner += 1;
            }

            res
        };

        TABLE[sq.0 as usize]
    }

    /// マスが盤面内かどうかを返す。
    pub const fn is_on_board(self) -> bool {
        const MASK: i32 = (1 << 13) | (1 << 18) | (1 << 23) | (1 << 28);

        (self.0 & MASK) == 0
    }

    /// `const` 文脈で必要なので。
    const fn to_square(self) -> Square {
        Square(self.0 & 0xFF)
    }

    /// `const` 文脈で必要なので。
    const fn add_delta(self, delta: i32) -> Self {
        Self(self.0 + delta)
    }
}

impl From<Square> for SquareWithWall {
    fn from(sq: Square) -> Self {
        Self::from_square(sq)
    }
}

impl From<SquareWithWall> for Square {
    fn from(sqww: SquareWithWall) -> Self {
        sqww.to_square()
    }
}

impl std::ops::Add<i32> for SquareWithWall {
    type Output = SquareWithWall;

    fn add(self, rhs: i32) -> SquareWithWall {
        self.add_delta(rhs)
    }
}

impl std::ops::Add<SquareWithWall> for i32 {
    type Output = SquareWithWall;

    fn add(self, rhs: SquareWithWall) -> SquareWithWall {
        SquareWithWall(self + rhs.0)
    }
}

impl std::ops::AddAssign<i32> for SquareWithWall {
    fn add_assign(&mut self, rhs: i32) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub<i32> for SquareWithWall {
    type Output = SquareWithWall;

    fn sub(self, rhs: i32) -> SquareWithWall {
        SquareWithWall(self.0 - rhs)
    }
}

impl std::ops::SubAssign<i32> for SquareWithWall {
    fn sub_assign(&mut self, rhs: i32) {
        *self = *self - rhs;
    }
}

impl std::fmt::Debug for SquareWithWall {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            SQWW_11 => write!(f, "SQWW_11"),
            SQWW_12 => write!(f, "SQWW_12"),
            SQWW_13 => write!(f, "SQWW_13"),
            SQWW_14 => write!(f, "SQWW_14"),
            SQWW_15 => write!(f, "SQWW_15"),
            SQWW_16 => write!(f, "SQWW_16"),
            SQWW_17 => write!(f, "SQWW_17"),
            SQWW_18 => write!(f, "SQWW_18"),
            SQWW_19 => write!(f, "SQWW_19"),
            SQWW_21 => write!(f, "SQWW_21"),
            SQWW_22 => write!(f, "SQWW_22"),
            SQWW_23 => write!(f, "SQWW_23"),
            SQWW_24 => write!(f, "SQWW_24"),
            SQWW_25 => write!(f, "SQWW_25"),
            SQWW_26 => write!(f, "SQWW_26"),
            SQWW_27 => write!(f, "SQWW_27"),
            SQWW_28 => write!(f, "SQWW_28"),
            SQWW_29 => write!(f, "SQWW_29"),
            SQWW_31 => write!(f, "SQWW_31"),
            SQWW_32 => write!(f, "SQWW_32"),
            SQWW_33 => write!(f, "SQWW_33"),
            SQWW_34 => write!(f, "SQWW_34"),
            SQWW_35 => write!(f, "SQWW_35"),
            SQWW_36 => write!(f, "SQWW_36"),
            SQWW_37 => write!(f, "SQWW_37"),
            SQWW_38 => write!(f, "SQWW_38"),
            SQWW_39 => write!(f, "SQWW_39"),
            SQWW_41 => write!(f, "SQWW_41"),
            SQWW_42 => write!(f, "SQWW_42"),
            SQWW_43 => write!(f, "SQWW_43"),
            SQWW_44 => write!(f, "SQWW_44"),
            SQWW_45 => write!(f, "SQWW_45"),
            SQWW_46 => write!(f, "SQWW_46"),
            SQWW_47 => write!(f, "SQWW_47"),
            SQWW_48 => write!(f, "SQWW_48"),
            SQWW_49 => write!(f, "SQWW_49"),
            SQWW_51 => write!(f, "SQWW_51"),
            SQWW_52 => write!(f, "SQWW_52"),
            SQWW_53 => write!(f, "SQWW_53"),
            SQWW_54 => write!(f, "SQWW_54"),
            SQWW_55 => write!(f, "SQWW_55"),
            SQWW_56 => write!(f, "SQWW_56"),
            SQWW_57 => write!(f, "SQWW_57"),
            SQWW_58 => write!(f, "SQWW_58"),
            SQWW_59 => write!(f, "SQWW_59"),
            SQWW_61 => write!(f, "SQWW_61"),
            SQWW_62 => write!(f, "SQWW_62"),
            SQWW_63 => write!(f, "SQWW_63"),
            SQWW_64 => write!(f, "SQWW_64"),
            SQWW_65 => write!(f, "SQWW_65"),
            SQWW_66 => write!(f, "SQWW_66"),
            SQWW_67 => write!(f, "SQWW_67"),
            SQWW_68 => write!(f, "SQWW_68"),
            SQWW_69 => write!(f, "SQWW_69"),
            SQWW_71 => write!(f, "SQWW_71"),
            SQWW_72 => write!(f, "SQWW_72"),
            SQWW_73 => write!(f, "SQWW_73"),
            SQWW_74 => write!(f, "SQWW_74"),
            SQWW_75 => write!(f, "SQWW_75"),
            SQWW_76 => write!(f, "SQWW_76"),
            SQWW_77 => write!(f, "SQWW_77"),
            SQWW_78 => write!(f, "SQWW_78"),
            SQWW_79 => write!(f, "SQWW_79"),
            SQWW_81 => write!(f, "SQWW_81"),
            SQWW_82 => write!(f, "SQWW_82"),
            SQWW_83 => write!(f, "SQWW_83"),
            SQWW_84 => write!(f, "SQWW_84"),
            SQWW_85 => write!(f, "SQWW_85"),
            SQWW_86 => write!(f, "SQWW_86"),
            SQWW_87 => write!(f, "SQWW_87"),
            SQWW_88 => write!(f, "SQWW_88"),
            SQWW_89 => write!(f, "SQWW_89"),
            SQWW_91 => write!(f, "SQWW_91"),
            SQWW_92 => write!(f, "SQWW_92"),
            SQWW_93 => write!(f, "SQWW_93"),
            SQWW_94 => write!(f, "SQWW_94"),
            SQWW_95 => write!(f, "SQWW_95"),
            SQWW_96 => write!(f, "SQWW_96"),
            SQWW_97 => write!(f, "SQWW_97"),
            SQWW_98 => write!(f, "SQWW_98"),
            SQWW_99 => write!(f, "SQWW_99"),
            _ => write!(f, "SquareWithWall({})", self.0),
        }
    }
}

/// 駒種(陣営の区別なし)。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PieceKind(u32);

// PieceKind の内部値はやねうら王からのパクリ。
// 値が連続で、かつ (1 << 3) を OR することで成駒になる。

pub const NO_PIECE_KIND: PieceKind = PieceKind(0);
pub const PAWN: PieceKind = PieceKind(1);
pub const LANCE: PieceKind = PieceKind(2);
pub const KNIGHT: PieceKind = PieceKind(3);
pub const SILVER: PieceKind = PieceKind(4);
pub const BISHOP: PieceKind = PieceKind(5);
pub const ROOK: PieceKind = PieceKind(6);
pub const GOLD: PieceKind = PieceKind(7);
pub const KING: PieceKind = PieceKind(8);
pub const PRO_PAWN: PieceKind = PieceKind(9);
pub const PRO_LANCE: PieceKind = PieceKind(10);
pub const PRO_KNIGHT: PieceKind = PieceKind(11);
pub const PRO_SILVER: PieceKind = PieceKind(12);
pub const HORSE: PieceKind = PieceKind(13);
pub const DRAGON: PieceKind = PieceKind(14);

impl PieceKind {
    /// 有効値かどうかを返す。`NO_PIECE_KIND` も有効とみなす。
    pub const fn is_valid(self) -> bool {
        NO_PIECE_KIND.0 <= self.0 && self.0 <= DRAGON.0
    }

    /// 有効値かつ実際の駒かどうかを返す。`NO_PIECE_KIND` は実際の駒ではない。
    pub const fn is_piece(self) -> bool {
        PAWN.0 <= self.0 && self.0 <= DRAGON.0
    }

    /// 成れる駒種かどうかを返す。
    pub const fn is_promotable(self) -> bool {
        PAWN.0 <= self.0 && self.0 <= ROOK.0
    }

    /// 成駒かどうかを返す。
    pub const fn is_promoted(self) -> bool {
        PRO_PAWN.0 <= self.0 && self.0 <= DRAGON.0
    }

    /// 手駒となりうる駒種かどうかを返す。
    pub const fn is_hand(self) -> bool {
        PAWN.0 <= self.0 && self.0 <= GOLD.0
    }

    /// 遠隔利きを持つ駒種かどうかを返す。香、角、飛車、馬、龍が該当する。
    pub const fn has_ranged_effect(self) -> bool {
        // 香以外はまとめて判定できる。内部値は以下の通りなので...
        //
        // 角:    5 = 0b0101
        // 飛車:  6 = 0b0110
        // 馬:   13 = 0b1101
        // 龍:   14 = 0b1110
        //
        // 1 を足して 0b110 でマスクした結果が 0b110 かどうかを見ればよい。

        self.0 == LANCE.0 || ((self.0 + 1) & 0b110) == 0b110
    }

    /// 成った駒種を返す。`self` は成れる駒種でなければならない。
    pub const fn to_promoted(self) -> Self {
        debug_assert!(self.is_promotable());

        Self(self.0 | (1 << 3))
    }

    /// 成っていない駒種を返す。`self` は玉であってはならない。
    pub const fn to_raw(self) -> Self {
        debug_assert!(self.0 != KING.0);

        Self(self.0 & 7)
    }

    /// 実際の駒である駒種を昇順に列挙する。
    pub fn iter_piece(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        // ExactSizeIterator にするため、配列をベタ書きする。
        [
            PAWN, LANCE, KNIGHT, SILVER, BISHOP, ROOK, GOLD, KING, PRO_PAWN, PRO_LANCE, PRO_KNIGHT,
            PRO_SILVER, HORSE, DRAGON,
        ]
        .into_iter()
    }

    /// 手駒となりうる駒種を昇順に列挙する。
    pub fn iter_hand(
    ) -> impl Iterator<Item = Self> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
        [PAWN, LANCE, KNIGHT, SILVER, BISHOP, ROOK, GOLD].into_iter()
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> u32 {
        self.0
    }
}

impl From<PieceKind> for u32 {
    fn from(pk: PieceKind) -> Self {
        pk.0
    }
}

impl From<PieceKind> for usize {
    fn from(pk: PieceKind) -> Self {
        debug_assert!(pk.is_valid());

        pk.0 as Self
    }
}

impl std::fmt::Debug for PieceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NO_PIECE_KIND => write!(f, "NO_PIECE_KIND"),
            PAWN => write!(f, "PAWN"),
            LANCE => write!(f, "LANCE"),
            KNIGHT => write!(f, "KNIGHT"),
            SILVER => write!(f, "SILVER"),
            BISHOP => write!(f, "BISHOP"),
            ROOK => write!(f, "ROOK"),
            GOLD => write!(f, "GOLD"),
            KING => write!(f, "KING"),
            PRO_PAWN => write!(f, "PRO_PAWN"),
            PRO_LANCE => write!(f, "PRO_LANCE"),
            PRO_KNIGHT => write!(f, "PRO_KNIGHT"),
            PRO_SILVER => write!(f, "PRO_SILVER"),
            HORSE => write!(f, "HORSE"),
            DRAGON => write!(f, "DRAGON"),
            _ => write!(f, "PieceKind({})", self.0),
        }
    }
}

impl std::fmt::Display for PieceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NO_PIECE_KIND => write!(f, "・"),
            PAWN => write!(f, "歩"),
            LANCE => write!(f, "香"),
            KNIGHT => write!(f, "桂"),
            SILVER => write!(f, "銀"),
            BISHOP => write!(f, "角"),
            ROOK => write!(f, "飛"),
            GOLD => write!(f, "金"),
            KING => write!(f, "玉"),
            PRO_PAWN => write!(f, "と"),
            PRO_LANCE => write!(f, "杏"),
            PRO_KNIGHT => write!(f, "圭"),
            PRO_SILVER => write!(f, "全"),
            HORSE => write!(f, "馬"),
            DRAGON => write!(f, "龍"),
            _ => write!(f, "無効な駒種({})", self.0),
        }
    }
}

/// 駒(陣営の区別あり)。
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Piece(u32);

// Piece の内部値はやねうら王からのパクリ。
// HUM の駒に (1 << 4) を OR することで COM の駒になる。

pub const NO_PIECE: Piece = Piece(0);
pub const H_PAWN: Piece = Piece(1);
pub const H_LANCE: Piece = Piece(2);
pub const H_KNIGHT: Piece = Piece(3);
pub const H_SILVER: Piece = Piece(4);
pub const H_BISHOP: Piece = Piece(5);
pub const H_ROOK: Piece = Piece(6);
pub const H_GOLD: Piece = Piece(7);
pub const H_KING: Piece = Piece(8);
pub const H_PRO_PAWN: Piece = Piece(9);
pub const H_PRO_LANCE: Piece = Piece(10);
pub const H_PRO_KNIGHT: Piece = Piece(11);
pub const H_PRO_SILVER: Piece = Piece(12);
pub const H_HORSE: Piece = Piece(13);
pub const H_DRAGON: Piece = Piece(14);
pub const C_PAWN: Piece = Piece(17);
pub const C_LANCE: Piece = Piece(18);
pub const C_KNIGHT: Piece = Piece(19);
pub const C_SILVER: Piece = Piece(20);
pub const C_BISHOP: Piece = Piece(21);
pub const C_ROOK: Piece = Piece(22);
pub const C_GOLD: Piece = Piece(23);
pub const C_KING: Piece = Piece(24);
pub const C_PRO_PAWN: Piece = Piece(25);
pub const C_PRO_LANCE: Piece = Piece(26);
pub const C_PRO_KNIGHT: Piece = Piece(27);
pub const C_PRO_SILVER: Piece = Piece(28);
pub const C_HORSE: Piece = Piece(29);
pub const C_DRAGON: Piece = Piece(30);

impl Piece {
    /// 陣営と駒種を指定して駒を作る。pk は実際の駒でなければならない。
    pub const fn new(side: Side, pk: PieceKind) -> Self {
        debug_assert!(pk.is_piece());

        Self((side.0 << 4) | pk.0)
    }

    /// 有効値かどうかを返す。`NO_PIECE` も有効とみなす。
    pub const fn is_valid(self) -> bool {
        NO_PIECE.0 <= self.0 && self.0 <= C_DRAGON.0
    }

    /// 有効値かつ実際の駒かどうかを返す。`NO_PIECE` は実際の駒ではない。
    pub const fn is_piece(self) -> bool {
        H_PAWN.0 <= self.0 && self.0 <= C_DRAGON.0
    }

    /// 成れる駒かどうかを返す。
    pub const fn is_promotable(self) -> bool {
        self.kind().is_promotable()
    }

    /// 成駒かどうかを返す。
    pub const fn is_promoted(self) -> bool {
        self.kind().is_promoted()
    }

    /// 遠隔利きを持つ駒かどうかを返す。香、角、飛車、馬、龍が該当する。
    pub const fn has_ranged_effect(self) -> bool {
        // PieceKind::has_ranged_effect() と同様。

        self.kind().0 == LANCE.0 || ((self.0 + 1) & 0b110) == 0b110
    }

    /// 所属陣営を返す。`self` は実際の駒でなければならない。
    pub const fn side(self) -> Side {
        debug_assert!(self.is_piece());

        Side((self.0 >> 4) & 1)
    }

    /// 駒種を返す。
    pub const fn kind(self) -> PieceKind {
        PieceKind(self.0 & 0xF)
    }

    /// 成った駒を返す。`self` は成れる駒でなければならない。
    pub const fn to_promoted(self) -> Self {
        debug_assert!(self.is_promotable());

        Self(self.0 | (1 << 3))
    }

    /// 成っていない駒種を返す。`self` は玉であってはならない。
    pub const fn to_raw_kind(self) -> PieceKind {
        debug_assert!(self.kind().0 != KING.0);

        PieceKind(self.0 & 7)
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> u32 {
        self.0
    }
}

impl From<Piece> for u32 {
    fn from(pc: Piece) -> Self {
        pc.0
    }
}

impl std::fmt::Debug for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NO_PIECE => write!(f, "NO_PIECE"),
            H_PAWN => write!(f, "H_PAWN"),
            H_LANCE => write!(f, "H_LANCE"),
            H_KNIGHT => write!(f, "H_KNIGHT"),
            H_SILVER => write!(f, "H_SILVER"),
            H_BISHOP => write!(f, "H_BISHOP"),
            H_ROOK => write!(f, "H_ROOK"),
            H_GOLD => write!(f, "H_GOLD"),
            H_KING => write!(f, "H_KING"),
            H_PRO_PAWN => write!(f, "H_PRO_PAWN"),
            H_PRO_LANCE => write!(f, "H_PRO_LANCE"),
            H_PRO_KNIGHT => write!(f, "H_PRO_KNIGHT"),
            H_PRO_SILVER => write!(f, "H_PRO_SILVER"),
            H_HORSE => write!(f, "H_HORSE"),
            H_DRAGON => write!(f, "H_DRAGON"),
            C_PAWN => write!(f, "C_PAWN"),
            C_LANCE => write!(f, "C_LANCE"),
            C_KNIGHT => write!(f, "C_KNIGHT"),
            C_SILVER => write!(f, "C_SILVER"),
            C_BISHOP => write!(f, "C_BISHOP"),
            C_ROOK => write!(f, "C_ROOK"),
            C_GOLD => write!(f, "C_GOLD"),
            C_KING => write!(f, "C_KING"),
            C_PRO_PAWN => write!(f, "C_PRO_PAWN"),
            C_PRO_LANCE => write!(f, "C_PRO_LANCE"),
            C_PRO_KNIGHT => write!(f, "C_PRO_KNIGHT"),
            C_PRO_SILVER => write!(f, "C_PRO_SILVER"),
            C_HORSE => write!(f, "C_HORSE"),
            C_DRAGON => write!(f, "C_DRAGON"),
            _ => write!(f, "Piece({})", self.0),
        }
    }
}

/// 指し手。
///
/// `u32` に pack されている。ビットレイアウトはやねうら王からのパクリ:
///
/// * bit0-6:   移動先
/// * bit7-13:  移動元(駒打ちなら打った駒種)
/// * bit14:    駒打ちか
/// * bit15:    成りか
///
/// `u16` にも収まるが、置換表を使わない場合サイズを切り詰める意義があるか微妙なので `u32` にしておく。
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Move(u32);

impl Move {
    const FLAG_DROP: u32 = 1 << 14;
    const FLAG_PROMOTION: u32 = 1 << 15;

    /// 盤上の駒を動かして成らない指し手を作る。
    ///
    /// `src` と `dst` は相異なる盤面内のマスでなければならない。
    pub const fn new_walk(src: Square, dst: Square) -> Self {
        debug_assert!(src.0 != dst.0);
        debug_assert!(src.is_on_board());
        debug_assert!(dst.is_on_board());

        Self((dst.0 as u32) | ((src.0 as u32) << 7))
    }

    /// 盤上の駒を動かして成る指し手を作る。
    ///
    /// `src` と `dst` は相異なる盤面内のマスでなければならない。
    pub const fn new_walk_promotion(src: Square, dst: Square) -> Self {
        debug_assert!(src.0 != dst.0);
        debug_assert!(src.is_on_board());
        debug_assert!(dst.is_on_board());

        Self((dst.0 as u32) | ((src.0 as u32) << 7) | Self::FLAG_PROMOTION)
    }

    /// 駒打ちの指し手を作る。
    ///
    /// `pk` は手駒となりうる駒種でなければならない。
    /// `dst` は盤面内のマスでなければならない。
    pub const fn new_drop(pk: PieceKind, dst: Square) -> Self {
        debug_assert!(pk.is_hand());
        debug_assert!(dst.is_on_board());

        Self((dst.0 as u32) | (pk.0 << 7) | Self::FLAG_DROP)
    }

    /// 指し手が有効かどうかを返す。盤面は考慮しない。
    ///
    /// 有効な指し手の定義は以下の通り:
    ///
    /// * 駒打ちフラグと成りフラグが同時に立っていない。
    /// * 盤上の駒を動かす場合、移動元と移動先が相異なる盤面内のマスである。
    /// * 駒打ちの場合、駒種が手駒となりうるものであり、かつ移動先が盤面内のマスである。
    pub const fn is_valid(self) -> bool {
        if self.is_drop() && self.is_promotion() {
            return false;
        }

        let dst = self.dst();

        if self.is_drop() {
            let pk = self.dropped_piece_kind();
            pk.is_hand() && dst.is_on_board()
        } else {
            let src = self.src();
            src.0 != dst.0 && src.is_on_board() && dst.is_on_board()
        }
    }

    /// 駒打ちかどうかを返す。
    pub const fn is_drop(self) -> bool {
        (self.0 & Self::FLAG_DROP) != 0
    }

    /// 成りかどうかを返す。
    pub const fn is_promotion(self) -> bool {
        (self.0 & Self::FLAG_PROMOTION) != 0
    }

    /// 移動先を返す。
    pub const fn dst(self) -> Square {
        Square((self.0 & 0x7F) as i32)
    }

    /// 移動元を返す。`self` は盤上の駒を動かす指し手でなければならない。
    pub const fn src(self) -> Square {
        debug_assert!(!self.is_drop());

        Square(((self.0 >> 7) & 0x7F) as i32)
    }

    /// 打った駒種を返す。`self` は駒打ちでなければならない。
    pub const fn dropped_piece_kind(self) -> PieceKind {
        debug_assert!(self.is_drop());

        PieceKind((self.0 >> 7) & 0x7F)
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        #[allow(dead_code)]
        #[derive(Debug)]
        enum MoveDebug {
            Walk {
                src: Square,
                dst: Square,
                promo: bool,
            },
            Drop {
                pk: PieceKind,
                dst: Square,
            },
        }

        if !self.is_valid() {
            return write!(f, "Move({})", self.0);
        }

        let mv_dbg = if self.is_drop() {
            MoveDebug::Drop {
                pk: self.dropped_piece_kind(),
                dst: self.dst(),
            }
        } else {
            MoveDebug::Walk {
                src: self.src(),
                dst: self.dst(),
                promo: self.is_promotion(),
            }
        };

        write!(f, "{:?}", mv_dbg)
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.is_valid() {
            return write!(f, "無効な指し手({})", self.0);
        }

        if self.is_drop() {
            write!(f, "{}{}打", self.dst(), self.dropped_piece_kind())?;
        } else {
            write!(f, "{}{}", self.src(), self.dst())?;
            if self.is_promotion() {
                f.write_str("成")?;
            }
        }

        Ok(())
    }
}

/// undo 可能な指し手。
///
/// `u32` に pack されている。ビットレイアウトは `Move` のそれを拡張したもの:
///
/// * bit0-6:   移動先
/// * bit7-13:  移動元(駒打ちなら打った駒種)
/// * bit14:    駒打ちか
/// * bit15:    成りか
/// * bit16-20: 移動元の駒(陣営の区別あり。駒打ちなら意味を持たない)
/// * bit21-25: 捕獲した駒(陣営の区別あり。駒取りでない場合 `NO_PIECE`, 即ち 0)
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct UndoableMove(u32);

impl UndoableMove {
    const FLAG_DROP: u32 = Move::FLAG_DROP;
    const FLAG_PROMOTION: u32 = Move::FLAG_PROMOTION;

    /// 駒を動かす指し手から `UndoableMove` を作る。
    ///
    /// 駒取りでない場合、`pc_captured` には NO_PIECE を渡す。
    pub const fn from_move_walk(mv: Move, pc_src: Piece, pc_captured: Piece) -> Self {
        Self(mv.0 | (pc_src.0 << 16) | (pc_captured.0 << 21))
    }

    /// 駒打ちの指し手から `UndoableMove` を作る。
    pub const fn from_move_drop(mv: Move) -> Self {
        // 駒打ちの場合、既に undo 用情報は揃っている。
        Self(mv.0)
    }

    /// 指し手が有効かどうかを返す。盤面は考慮しない。
    ///
    /// 有効な指し手の定義は以下の通り:
    ///
    /// * 駒打ちフラグと成りフラグが同時に立っていない。
    /// * 盤上の駒を動かす場合、以下の全ての条件を満たす:
    ///   - 移動元と移動先が相異なる盤面内のマスである。
    ///   - 移動元の駒が実際の駒である。
    ///   - 捕獲した駒がある場合、それは玉ではなく、かつ移動元の駒と逆の陣営に属する。
    /// * 駒打ちの場合、以下の全ての条件を満たす:
    ///   - 駒種が手駒となりうるものである。
    ///   - 移動先が盤面内のマスである。
    ///   - 捕獲した駒がない。
    pub const fn is_valid(self) -> bool {
        if self.is_drop() && self.is_promotion() {
            return false;
        }

        let dst = self.dst();

        if self.is_drop() {
            let pk = self.dropped_piece_kind();
            pk.is_hand() && dst.is_on_board() && self.piece_captured().0 == NO_PIECE.0
        } else {
            let src = self.src();
            let pc_src = self.piece_src();
            let pc_captured = self.piece_captured();
            if src.0 == dst.0 || !src.is_on_board() || !dst.is_on_board() {
                return false;
            }
            if !pc_src.is_piece() {
                return false;
            }
            if pc_captured.0 != NO_PIECE.0 {
                if pc_captured.kind().0 == KING.0 {
                    return false;
                }
                if pc_captured.side().0 == pc_src.side().0 {
                    return false;
                }
            }
            true
        }
    }

    /// 駒打ちかどうかを返す。
    pub const fn is_drop(self) -> bool {
        (self.0 & Self::FLAG_DROP) != 0
    }

    /// 成りかどうかを返す。
    pub const fn is_promotion(self) -> bool {
        (self.0 & Self::FLAG_PROMOTION) != 0
    }

    /// 移動先を返す。
    pub const fn dst(self) -> Square {
        Square((self.0 & 0x7F) as i32)
    }

    /// 移動元を返す。`self` は盤上の駒を動かす指し手でなければならない。
    pub const fn src(self) -> Square {
        debug_assert!(!self.is_drop());

        Square(((self.0 >> 7) & 0x7F) as i32)
    }

    /// 移動元の駒を返す。`self` は盤上の駒を動かす指し手でなければならない。
    pub const fn piece_src(self) -> Piece {
        debug_assert!(!self.is_drop());

        Piece((self.0 >> 16) & 0x1F)
    }

    /// 捕獲した駒(駒取りでない場合 NO_PIECE)を返す。
    pub const fn piece_captured(self) -> Piece {
        Piece((self.0 >> 21) & 0x1F)
    }

    /// 移動後の駒を返す。`self` は盤上の駒を動かす指し手でなければならない。
    ///
    /// 移動元の駒と成りフラグから計算される。
    pub const fn piece_dst(self) -> Piece {
        debug_assert!(!self.is_drop());

        let pc_src = self.piece_src();
        if self.is_promotion() {
            pc_src.to_promoted()
        } else {
            pc_src
        }
    }

    /// 打った駒種を返す。`self` は駒打ちでなければならない。
    pub const fn dropped_piece_kind(self) -> PieceKind {
        debug_assert!(self.is_drop());

        PieceKind((self.0 >> 7) & 0x7F)
    }
}

impl From<UndoableMove> for Move {
    #[inline]
    fn from(umv: UndoableMove) -> Self {
        Self(umv.0 & 0xFFFF)
    }
}

impl std::fmt::Debug for UndoableMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        #[allow(dead_code)]
        #[derive(Debug)]
        enum UndoableMoveDebug {
            Walk {
                src: Square,
                dst: Square,
                promo: bool,
                pc_src: Piece,
                pc_captured: Piece,
            },
            Drop {
                pk: PieceKind,
                dst: Square,
            },
        }

        if !self.is_valid() {
            return write!(f, "UndoableMove({})", self.0);
        }

        let umv_dbg = if self.is_drop() {
            UndoableMoveDebug::Drop {
                pk: self.dropped_piece_kind(),
                dst: self.dst(),
            }
        } else {
            UndoableMoveDebug::Walk {
                src: self.src(),
                dst: self.dst(),
                promo: self.is_promotion(),
                pc_src: self.piece_src(),
                pc_captured: self.piece_captured(),
            }
        };

        write!(f, "{:?}", umv_dbg)
    }
}

impl std::fmt::Display for UndoableMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.is_valid() {
            return write!(f, "無効な指し手({})", self.0);
        }

        if self.is_drop() {
            write!(f, "{}{}打", self.dst(), self.dropped_piece_kind())?;
        } else {
            write!(f, "{}{}{}", self.src(), self.dst(), self.piece_src().kind())?;
            if self.is_promotion() {
                f.write_str("成")?;
            }
            if self.piece_captured() != NO_PIECE {
                write!(f, " (捕獲: {})", self.piece_captured().kind())?;
            }
        }

        Ok(())
    }
}

/// 盤面。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Board([Piece; 81]);

impl Board {
    /// 空の盤面を返す。
    pub const fn empty() -> Self {
        Self([NO_PIECE; 81])
    }

    /// 平手初期盤面を返す。
    pub const fn startpos() -> Self {
        #[rustfmt::skip]
        const INNER: [Piece; 81] = [
            C_LANCE,  NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_LANCE,
            C_KNIGHT, C_BISHOP, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, H_ROOK,   H_KNIGHT,
            C_SILVER, NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_SILVER,
            C_GOLD,   NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_GOLD,
            C_KING,   NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_KING,
            C_GOLD,   NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_GOLD,
            C_SILVER, NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_SILVER,
            C_KNIGHT, C_ROOK,   C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, H_BISHOP, H_KNIGHT,
            C_LANCE,  NO_PIECE, C_PAWN, NO_PIECE, NO_PIECE, NO_PIECE, H_PAWN, NO_PIECE, H_LANCE,
        ];

        Self(INNER)
    }
}

impl std::ops::Index<Square> for Board {
    type Output = Piece;

    fn index(&self, sq: Square) -> &Self::Output {
        unsafe { self.0.get_unchecked(usize::from(sq)) }
    }
}

impl std::ops::IndexMut<Square> for Board {
    fn index_mut(&mut self, sq: Square) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(usize::from(sq)) }
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for row in Row::iter() {
            for col in Col::iter().rev() {
                let sq = Square::from_col_row(col, row);
                let pc = self[sq];
                if pc == NO_PIECE || pc.side() == HUM {
                    f.write_str(" ")?;
                } else {
                    f.write_str("v")?;
                }
                write!(f, "{}", pc.kind())?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

/// 手駒。
///
/// とりあえず単純な配列とする(優等局面判定などは不要なので)。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Hand([u32; 8]);

impl Hand {
    /// 空の手駒を返す。
    pub const fn empty() -> Self {
        Self([0; 8])
    }

    /// 手駒が空かどうかを返す。
    pub const fn is_empty(&self) -> bool {
        let mut i = 0;
        while i < self.0.len() {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }

        true
    }
}

impl std::ops::Index<PieceKind> for Hand {
    type Output = u32;

    /// 手駒とならない駒種を渡してはならない。
    fn index(&self, pk: PieceKind) -> &Self::Output {
        debug_assert!(pk.is_hand());

        &self.0[usize::from(pk)]
    }
}

impl std::ops::IndexMut<PieceKind> for Hand {
    /// 手駒とならない駒種を渡してはならない。
    fn index_mut(&mut self, pk: PieceKind) -> &mut Self::Output {
        debug_assert!(pk.is_hand());

        &mut self.0[usize::from(pk)]
    }
}

impl std::fmt::Display for Hand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        const PKS: [PieceKind; 7] = [ROOK, BISHOP, GOLD, SILVER, KNIGHT, LANCE, PAWN];

        for pk in PKS {
            let n = self[pk];
            if n == 0 {
                continue;
            }

            write!(f, "{}", pk)?;
            if n >= 2 {
                write!(f, "{}", n)?;
            }
        }

        Ok(())
    }
}

/// 両陣営の手駒。`Side` でインデックスアクセスできる。
pub type Hands = MyArray1<Hand, Side, 2>;

/// 方向。
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Direction(u32);

impl Direction {
    pub const RU: Direction = Direction(0);
    pub const R: Direction = Direction(1);
    pub const RD: Direction = Direction(2);
    pub const U: Direction = Direction(3);
    pub const D: Direction = Direction(4);
    pub const LU: Direction = Direction(5);
    pub const L: Direction = Direction(6);
    pub const LD: Direction = Direction(7);

    /// 内部値を指定して方向を作る。無効値を渡してはならない。
    pub const fn from_inner(inner: u32) -> Self {
        let this = Self(inner);
        debug_assert!(this.is_valid());

        this
    }

    /// 有効値かどうかを返す。
    pub const fn is_valid(self) -> bool {
        Self::RU.0 <= self.0 && self.0 <= Self::LD.0
    }

    /// 逆方向を返す。
    pub const fn inv(self) -> Self {
        // 内部値は以下のように割り当てられているので、逆方向同士を加えると常に 7 になる。
        //
        // 530
        // 6.1
        // 742

        Self(7 - self.0)
    }

    /// 方向を `SquareWithWall` の差分値に変換する。
    pub const fn to_sqww_delta(self) -> i32 {
        const TABLE: [i32; 8] = [
            SquareWithWall::DIR_RU,
            SquareWithWall::DIR_R,
            SquareWithWall::DIR_RD,
            SquareWithWall::DIR_U,
            SquareWithWall::DIR_D,
            SquareWithWall::DIR_LU,
            SquareWithWall::DIR_L,
            SquareWithWall::DIR_LD,
        ];

        TABLE[self.0 as usize]
    }
}

impl From<Direction> for u32 {
    fn from(dir: Direction) -> Self {
        dir.0
    }
}

impl From<Direction> for usize {
    fn from(dir: Direction) -> Self {
        debug_assert!(dir.is_valid());

        dir.0 as Self
    }
}

impl std::fmt::Debug for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::RU => write!(f, "Direction::RU"),
            Self::R => write!(f, "Direction::R"),
            Self::RD => write!(f, "Direction::RD"),
            Self::U => write!(f, "Direction::U"),
            Self::D => write!(f, "Direction::D"),
            Self::LU => write!(f, "Direction::LU"),
            Self::L => write!(f, "Direction::L"),
            Self::LD => write!(f, "Direction::LD"),
            _ => write!(f, "Direction({})", self.0),
        }
    }
}

/// 8 方向の集合。
///
/// * bit0: RU
/// * bit1: R
/// * bit2: RD
/// * bit3: U
/// * bit4: D
/// * bit5: LU
/// * bit6: L
/// * bit7: LD
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct DirectionSet(u8);

impl DirectionSet {
    pub const RU: DirectionSet = DirectionSet(1 << 0);
    pub const R: DirectionSet = DirectionSet(1 << 1);
    pub const RD: DirectionSet = DirectionSet(1 << 2);
    pub const U: DirectionSet = DirectionSet(1 << 3);
    pub const D: DirectionSet = DirectionSet(1 << 4);
    pub const LU: DirectionSet = DirectionSet(1 << 5);
    pub const L: DirectionSet = DirectionSet(1 << 6);
    pub const LD: DirectionSet = DirectionSet(1 << 7);

    /// 空の(どの方向も含まない) `DirectionSet` を作る。
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 全ての方向を含む `DirectionSet` を作る。
    pub const fn all() -> Self {
        Self(0xFF)
    }

    /// 内部値を指定して `DirectionSet` を作る。
    pub const fn from_inner(inner: u8) -> Self {
        Self(inner)
    }

    /// 単一の方向のみを含む `DirectionSet` を返す。(`const` 文脈で必要)
    const fn from_direction(dir: Direction) -> Self {
        Self(1 << dir.0)
    }

    /// `src` から `dst` を見たときの方向を返す。
    ///
    /// 戻り値は高々 1 つの方向しか含まない。
    /// `src == dst` の場合、空集合を返す。
    pub const fn from_squares(src: Square, dst: Square) -> Self {
        const TABLE: [[DirectionSet; 81]; 81] = {
            let mut res = [[DirectionSet::empty(); 81]; 81];

            let mut src_i = 0;
            while src_i < 81 {
                let src = Square(src_i);

                let mut dir_i = 0;
                while dir_i < 8 {
                    let dir = Direction(dir_i);

                    // dir 方向へ盤面外に出るまで進みつつ、方向を記録する。
                    let delta = dir.to_sqww_delta();
                    let mut dst_ww = SquareWithWall::from_square(src).add_delta(delta);
                    while dst_ww.is_on_board() {
                        let dst = dst_ww.to_square();
                        res[src.0 as usize][dst.0 as usize] = DirectionSet::from_direction(dir);

                        dst_ww = dst_ww.add_delta(delta);
                    }
                    dir_i += 1;
                }
                src_i += 1;
            }

            res
        };

        TABLE[src.0 as usize][dst.0 as usize]
    }

    /// 指定した駒について、影の利きの対象となる方向を含む `DirectionSet` を返す。
    /// 桂、玉に対しては空集合を返す。
    pub const fn from_piece_supported(pc: Piece) -> Self {
        const BISHOP_DIRS: DirectionSet = DirectionSet::RU
            .or(DirectionSet::RD)
            .or(DirectionSet::LU)
            .or(DirectionSet::LD);
        const ROOK_DIRS: DirectionSet = DirectionSet::R
            .or(DirectionSet::U)
            .or(DirectionSet::D)
            .or(DirectionSet::L);
        const H_SILVER_DIRS: DirectionSet = DirectionSet::RU
            .or(DirectionSet::RD)
            .or(DirectionSet::U)
            .or(DirectionSet::LU)
            .or(DirectionSet::LD);
        const H_GOLD_DIRS: DirectionSet = DirectionSet::RU
            .or(DirectionSet::R)
            .or(DirectionSet::U)
            .or(DirectionSet::D)
            .or(DirectionSet::LU)
            .or(DirectionSet::L);
        const C_SILVER_DIRS: DirectionSet = DirectionSet::RU
            .or(DirectionSet::RD)
            .or(DirectionSet::D)
            .or(DirectionSet::LU)
            .or(DirectionSet::LD);
        const C_GOLD_DIRS: DirectionSet = DirectionSet::R
            .or(DirectionSet::RD)
            .or(DirectionSet::U)
            .or(DirectionSet::D)
            .or(DirectionSet::L)
            .or(DirectionSet::LD);

        const TABLE: [DirectionSet; 32] = [
            DirectionSet::empty(), // NO_PIECE
            DirectionSet::U,       // H_PAWN
            DirectionSet::U,       // H_LANCE
            DirectionSet::empty(), // H_KNIGHT
            H_SILVER_DIRS,         // H_SILVER
            BISHOP_DIRS,           // H_BISHOP
            ROOK_DIRS,             // H_ROOK
            H_GOLD_DIRS,           // H_GOLD
            DirectionSet::empty(), // H_KING
            H_GOLD_DIRS,           // H_PRO_PAWN
            H_GOLD_DIRS,           // H_PRO_LANCE
            H_GOLD_DIRS,           // H_PRO_KNIGHT
            H_GOLD_DIRS,           // H_PRO_SILVER
            DirectionSet::all(),   // H_HORSE
            DirectionSet::all(),   // H_DRAGON
            DirectionSet::empty(), // (15)
            DirectionSet::empty(), // (16)
            DirectionSet::D,       // C_PAWN
            DirectionSet::D,       // C_LANCE
            DirectionSet::empty(), // C_KNIGHT
            C_SILVER_DIRS,         // C_SILVER
            BISHOP_DIRS,           // C_BISHOP
            ROOK_DIRS,             // C_ROOK
            C_GOLD_DIRS,           // C_GOLD
            DirectionSet::empty(), // C_KING
            C_GOLD_DIRS,           // C_PRO_PAWN
            C_GOLD_DIRS,           // C_PRO_LANCE
            C_GOLD_DIRS,           // C_PRO_KNIGHT
            C_GOLD_DIRS,           // C_PRO_SILVER
            DirectionSet::all(),   // C_HORSE
            DirectionSet::all(),   // C_DRAGON
            DirectionSet::empty(), // (31)
        ];

        TABLE[pc.inner() as usize]
    }

    /// `self` が空かどうかを返す。
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `self` と `other` の共通部分が空かどうかを返す。
    pub const fn is_disjoint(self, other: Self) -> bool {
        self.and(other).is_empty()
    }

    /// `self` が `other` の subset かどうかを返す。
    pub const fn is_subset(self, other: Self) -> bool {
        self.and(other).0 == self.0
    }

    /// `self` が `other` の superset かどうかを返す。
    pub const fn is_superset(self, other: Self) -> bool {
        other.is_subset(self)
    }

    /// 指定した方向を含むかどうかを返す。
    pub const fn contains(self, dir: Direction) -> bool {
        (self.0 & (1 << dir.0)) != 0
    }

    /// 含まれる方向のうち、内部値が最小のものを得る。`self` は空であってはならない。
    pub fn get_least(self) -> Direction {
        let inner = bitop::lsb_u8(self.0);
        Direction(inner)
    }

    /// 含まれる方向のうち、内部値が最小のものを pop する。`self` は空であってはならない。
    pub fn pop_least(&mut self) -> Direction {
        let inner = bitop::pop_lsb_u8(&mut self.0);
        Direction(inner)
    }

    /// 含まれる全ての方向について `f` を呼ぶ。
    pub fn for_each<F>(self, mut f: F)
    where
        F: FnMut(Direction),
    {
        let mut dirs = self;
        while !dirs.is_empty() {
            let dir = dirs.pop_least();
            f(dir);
        }
    }

    /// NOT 演算。`const` 文脈で使えるのが `!` 演算子との違い。
    pub const fn not(self) -> Self {
        Self(!self.0)
    }

    /// AND 演算。`const` 文脈で使えるのが `&` 演算子との違い。
    pub const fn and(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }

    /// OR 演算。`const` 文脈で使えるのが '|' 演算子との違い。
    pub const fn or(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }

    /// XOR 演算。`const` 文脈で使えるのが '^' 演算子との違い。
    pub const fn xor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }

    /// 内部値を返す。`const` 文脈で使える。
    pub const fn inner(self) -> u8 {
        self.0
    }
}

impl From<Direction> for DirectionSet {
    /// 単一の方向のみを含む `DirectionSet` を返す。
    fn from(dir: Direction) -> Self {
        Self::from_direction(dir)
    }
}

impl std::ops::Not for DirectionSet {
    type Output = Self;

    fn not(self) -> Self {
        self.not()
    }
}

impl std::ops::BitAnd for DirectionSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        self.and(rhs)
    }
}

impl std::ops::BitAndAssign for DirectionSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr for DirectionSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.or(rhs)
    }
}

impl std::ops::BitOrAssign for DirectionSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitXor for DirectionSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        self.xor(rhs)
    }
}

impl std::ops::BitXorAssign for DirectionSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl From<DirectionSet> for u8 {
    fn from(dirs: DirectionSet) -> Self {
        dirs.0
    }
}

impl From<DirectionSet> for usize {
    fn from(dirs: DirectionSet) -> Self {
        Self::from(dirs.0)
    }
}

impl std::fmt::Debug for DirectionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        const TABLE: [(DirectionSet, &str); 8] = [
            (DirectionSet::RU, "DirectionSet::RU"),
            (DirectionSet::R, "DirectionSet::R"),
            (DirectionSet::RD, "DirectionSet::RD"),
            (DirectionSet::U, "DirectionSet::U"),
            (DirectionSet::D, "DirectionSet::D"),
            (DirectionSet::LU, "DirectionSet::LU"),
            (DirectionSet::L, "DirectionSet::L"),
            (DirectionSet::LD, "DirectionSet::LD"),
        ];

        if self.is_empty() {
            return write!(f, "DirectionSet({})", self.0);
        }

        let mut first = true;
        let mut write_name = move |name: &str| -> std::fmt::Result {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str(name)?;
            first = false;
            Ok(())
        };

        for (dir, name) in TABLE {
            if !self.is_disjoint(dir) {
                write_name(name)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn test_row_is_promotion_zone() {
        assert!(ROW_1.is_promotion_zone(HUM));
        assert!(ROW_2.is_promotion_zone(HUM));
        assert!(ROW_3.is_promotion_zone(HUM));
        assert!(!ROW_4.is_promotion_zone(HUM));
        assert!(!ROW_5.is_promotion_zone(HUM));
        assert!(!ROW_6.is_promotion_zone(HUM));
        assert!(!ROW_7.is_promotion_zone(HUM));
        assert!(!ROW_8.is_promotion_zone(HUM));
        assert!(!ROW_9.is_promotion_zone(HUM));

        assert!(!ROW_1.is_promotion_zone(COM));
        assert!(!ROW_2.is_promotion_zone(COM));
        assert!(!ROW_3.is_promotion_zone(COM));
        assert!(!ROW_4.is_promotion_zone(COM));
        assert!(!ROW_5.is_promotion_zone(COM));
        assert!(!ROW_6.is_promotion_zone(COM));
        assert!(ROW_7.is_promotion_zone(COM));
        assert!(ROW_8.is_promotion_zone(COM));
        assert!(ROW_9.is_promotion_zone(COM));
    }

    #[test]
    fn test_square_distance() {
        assert_eq!(SQ_11.distance(SQ_11), 0);
        assert_eq!(SQ_23.distance(SQ_33), 1);
        assert_eq!(SQ_23.distance(SQ_34), 1);
        assert_eq!(SQ_75.distance(SQ_83), 2);
        assert_eq!(SQ_75.distance(SQ_34), 4);
        assert_eq!(SQ_91.distance(SQ_19), 8);
    }

    #[test]
    fn test_direction_set_basic() {
        assert!(DirectionSet::empty().is_empty());

        assert_eq!(DirectionSet::from(Direction::D), DirectionSet::D);

        const DIRS: DirectionSet = DirectionSet::RU.or(DirectionSet::D).or(DirectionSet::L);

        assert!(!DIRS.is_disjoint(DIRS));
        assert!(DIRS.is_disjoint(
            DirectionSet::R
                | DirectionSet::RD
                | DirectionSet::U
                | DirectionSet::LU
                | DirectionSet::LD
        ));
        assert!(!DIRS.is_disjoint(DirectionSet::RU | DirectionSet::U));

        assert!(DIRS.is_subset(DIRS));
        assert!(
            DIRS.is_subset(DirectionSet::RU | DirectionSet::R | DirectionSet::D | DirectionSet::L)
        );
        assert!(!DIRS.is_subset(DirectionSet::RU | DirectionSet::U));

        assert!(DIRS.is_superset(DIRS));
        assert!(DIRS.is_superset(DirectionSet::RU));
        assert!(!DIRS.is_superset(DirectionSet::RU | DirectionSet::U));
    }

    #[test]
    fn test_direction_set_bitop() {
        const DIRS: DirectionSet = DirectionSet::RU.or(DirectionSet::D).or(DirectionSet::L);

        assert_eq!(
            !DIRS,
            DirectionSet::R
                | DirectionSet::RD
                | DirectionSet::U
                | DirectionSet::LU
                | DirectionSet::LD
        );

        assert_eq!(
            DIRS & (DirectionSet::RU | DirectionSet::D | DirectionSet::LD),
            DirectionSet::RU | DirectionSet::D
        );

        assert_eq!(
            DIRS | (DirectionSet::RU | DirectionSet::LD),
            DirectionSet::RU | DirectionSet::D | DirectionSet::L | DirectionSet::LD
        );

        assert_eq!(
            DIRS ^ (DirectionSet::RU | DirectionSet::LD),
            DirectionSet::D | DirectionSet::L | DirectionSet::LD
        );
    }

    #[test]
    fn test_direction_set_from_squares() {
        assert_eq!(
            DirectionSet::from_squares(SQ_11, SQ_11),
            DirectionSet::empty()
        );

        assert_eq!(
            DirectionSet::from_squares(SQ_45, SQ_33),
            DirectionSet::empty()
        );

        assert_eq!(DirectionSet::from_squares(SQ_99, SQ_11), DirectionSet::RU);
        assert_eq!(DirectionSet::from_squares(SQ_95, SQ_15), DirectionSet::R);
        assert_eq!(DirectionSet::from_squares(SQ_91, SQ_19), DirectionSet::RD);
        assert_eq!(DirectionSet::from_squares(SQ_59, SQ_51), DirectionSet::U);
        assert_eq!(DirectionSet::from_squares(SQ_51, SQ_59), DirectionSet::D);
        assert_eq!(DirectionSet::from_squares(SQ_19, SQ_91), DirectionSet::LU);
        assert_eq!(DirectionSet::from_squares(SQ_15, SQ_95), DirectionSet::L);
        assert_eq!(DirectionSet::from_squares(SQ_11, SQ_99), DirectionSet::LD);

        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_23), DirectionSet::RU);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_24), DirectionSet::R);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_25), DirectionSet::RD);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_33), DirectionSet::U);
        assert_eq!(
            DirectionSet::from_squares(SQ_34, SQ_34),
            DirectionSet::empty()
        );
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_35), DirectionSet::D);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_43), DirectionSet::LU);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_44), DirectionSet::L);
        assert_eq!(DirectionSet::from_squares(SQ_34, SQ_45), DirectionSet::LD);
    }

    #[test]
    fn test_direction_set_get_least() {
        const DIRS: DirectionSet = DirectionSet::RU.or(DirectionSet::D).or(DirectionSet::L);

        assert_eq!(DIRS.get_least(), Direction::RU);
    }

    #[test]
    fn test_direction_set_pop_least() {
        let mut dirs = DirectionSet::RU | DirectionSet::D | DirectionSet::L;

        assert_eq!(dirs.pop_least(), Direction::RU);
        assert_eq!(dirs.pop_least(), Direction::D);
        assert_eq!(dirs.pop_least(), Direction::L);

        assert_eq!(dirs, DirectionSet::empty());
    }
}
