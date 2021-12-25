mod com;
mod general;

pub use self::com::*;
pub use self::general::*;

use arrayvec::ArrayVec;

use crate::shogi::Move;

/// 指し手配列。
///
/// 将棋の最大分岐数は 593 だが、一応もう少し余裕をもたせておく。
///
/// ref: [将棋における最大分岐数](https://www.nara-wu.ac.jp/math/personal/shinoda/bunki.html)
pub type MoveArray = ArrayVec<Move, 600>;
