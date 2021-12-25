//! 原作固有の要素。

use std::iter::FusedIterator;

use anyhow::bail;
use clap::arg_enum;

use crate::bbs;
use crate::position::Position;
use crate::sfen::sfen_decode_position;
use crate::shogi::*;

arg_enum! {
    /// 手合割(戦型指定含む)。
    ///
    /// 原作では、平手の場合時間制限なしなら四間飛車、ありなら中飛車になる。
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub enum Handicap {
        HumSenteSikenbisha,
        HumSenteNakabisha,
        HumHishaochi,
        HumNimaiochi,
        ComSenteSikenbisha,
        ComSenteNakabisha,
        ComHishaochi,
        ComNimaiochi,
    }
}

impl Handicap {
    /// 開始局面および時間制限設定に対応する手合割を返す。
    /// 対応する手合割が見つからなければエラーを返す。
    pub fn from_startpos(
        side_to_move: Side,
        board: &Board,
        hands: &Hands,
        timelimit: bool,
    ) -> anyhow::Result<Self> {
        macro_rules! pos_eq {
            ($handicap:expr) => {{
                let (s, b, h) = $handicap.startpos();
                (side_to_move, board, hands) == (s, &b, &h)
            }};
        }

        let this = if pos_eq!(Self::HumSenteSikenbisha) {
            if timelimit {
                Self::HumSenteNakabisha
            } else {
                Self::HumSenteSikenbisha
            }
        } else if pos_eq!(Self::HumHishaochi) {
            Self::HumHishaochi
        } else if pos_eq!(Self::HumNimaiochi) {
            Self::HumNimaiochi
        } else if pos_eq!(Self::ComSenteSikenbisha) {
            if timelimit {
                Self::ComSenteNakabisha
            } else {
                Self::ComSenteSikenbisha
            }
        } else if pos_eq!(Self::ComHishaochi) {
            Self::ComHishaochi
        } else if pos_eq!(Self::ComNimaiochi) {
            Self::ComNimaiochi
        } else {
            bail!("no handicap matches");
        };

        Ok(this)
    }

    /// 手合割に対応する開始局面の sfen 局面文字列を返す。
    pub const fn sfen(self) -> &'static str {
        match self {
            Self::HumSenteSikenbisha | Self::HumSenteNakabisha => "startpos",
            Self::HumHishaochi => {
                "sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B7/LNSGKGSNL b - 1"
            }
            Self::HumNimaiochi => {
                "sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/9/LNSGKGSNL b - 1"
            }
            Self::ComSenteSikenbisha | Self::ComSenteNakabisha => {
                "sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL w - 1"
            }
            Self::ComHishaochi => {
                "sfen lnsgkgsnl/7b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL w - 1"
            }
            Self::ComNimaiochi => {
                "sfen lnsgkgsnl/9/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL w - 1"
            }
        }
    }

    /// 手合割に対応する開始局面を返す。
    pub fn startpos(self) -> (Side, Board, Hands) {
        let (side_to_move, board, hands) = sfen_decode_position(self.sfen())
            .expect("Handicap::sfen() should return a valid sfen position string");

        (side_to_move, board, hands)
    }

    /// この手合割において先手となる陣営を返す。
    pub fn side_to_move(self) -> Side {
        self.startpos().0
    }
}

/// 原作準拠の内部値をマスに変換する。存在しないマスを表す値ならば `None` を返す。
pub fn naitou_square_from_value(value: u8) -> Option<Square> {
    (value != 99).then(|| {
        let col = COL_9 + 1 - i32::from(value % 11);
        let row = ROW_1 - 1 + i32::from(value / 11);

        Square::from_col_row(col, row)
    })
}

/// マスを原作準拠の内部値に変換する。
pub const fn naitou_square_to_value(sq: Square) -> u8 {
    #[allow(clippy::identity_op)]
    const TABLE: [u8; 81] = [
        11 * 1 + (10 - 1), // SQ_11
        11 * 2 + (10 - 1), // SQ_12
        11 * 3 + (10 - 1), // SQ_13
        11 * 4 + (10 - 1), // SQ_14
        11 * 5 + (10 - 1), // SQ_15
        11 * 6 + (10 - 1), // SQ_16
        11 * 7 + (10 - 1), // SQ_17
        11 * 8 + (10 - 1), // SQ_18
        11 * 9 + (10 - 1), // SQ_19
        11 * 1 + (10 - 2), // SQ_21
        11 * 2 + (10 - 2), // SQ_22
        11 * 3 + (10 - 2), // SQ_23
        11 * 4 + (10 - 2), // SQ_24
        11 * 5 + (10 - 2), // SQ_25
        11 * 6 + (10 - 2), // SQ_26
        11 * 7 + (10 - 2), // SQ_27
        11 * 8 + (10 - 2), // SQ_28
        11 * 9 + (10 - 2), // SQ_29
        11 * 1 + (10 - 3), // SQ_31
        11 * 2 + (10 - 3), // SQ_32
        11 * 3 + (10 - 3), // SQ_33
        11 * 4 + (10 - 3), // SQ_34
        11 * 5 + (10 - 3), // SQ_35
        11 * 6 + (10 - 3), // SQ_36
        11 * 7 + (10 - 3), // SQ_37
        11 * 8 + (10 - 3), // SQ_38
        11 * 9 + (10 - 3), // SQ_39
        11 * 1 + (10 - 4), // SQ_41
        11 * 2 + (10 - 4), // SQ_42
        11 * 3 + (10 - 4), // SQ_43
        11 * 4 + (10 - 4), // SQ_44
        11 * 5 + (10 - 4), // SQ_45
        11 * 6 + (10 - 4), // SQ_46
        11 * 7 + (10 - 4), // SQ_47
        11 * 8 + (10 - 4), // SQ_48
        11 * 9 + (10 - 4), // SQ_49
        11 * 1 + (10 - 5), // SQ_51
        11 * 2 + (10 - 5), // SQ_52
        11 * 3 + (10 - 5), // SQ_53
        11 * 4 + (10 - 5), // SQ_54
        11 * 5 + (10 - 5), // SQ_55
        11 * 6 + (10 - 5), // SQ_56
        11 * 7 + (10 - 5), // SQ_57
        11 * 8 + (10 - 5), // SQ_58
        11 * 9 + (10 - 5), // SQ_59
        11 * 1 + (10 - 6), // SQ_61
        11 * 2 + (10 - 6), // SQ_62
        11 * 3 + (10 - 6), // SQ_63
        11 * 4 + (10 - 6), // SQ_64
        11 * 5 + (10 - 6), // SQ_65
        11 * 6 + (10 - 6), // SQ_66
        11 * 7 + (10 - 6), // SQ_67
        11 * 8 + (10 - 6), // SQ_68
        11 * 9 + (10 - 6), // SQ_69
        11 * 1 + (10 - 7), // SQ_71
        11 * 2 + (10 - 7), // SQ_72
        11 * 3 + (10 - 7), // SQ_73
        11 * 4 + (10 - 7), // SQ_74
        11 * 5 + (10 - 7), // SQ_75
        11 * 6 + (10 - 7), // SQ_76
        11 * 7 + (10 - 7), // SQ_77
        11 * 8 + (10 - 7), // SQ_78
        11 * 9 + (10 - 7), // SQ_79
        11 * 1 + (10 - 8), // SQ_81
        11 * 2 + (10 - 8), // SQ_82
        11 * 3 + (10 - 8), // SQ_83
        11 * 4 + (10 - 8), // SQ_84
        11 * 5 + (10 - 8), // SQ_85
        11 * 6 + (10 - 8), // SQ_86
        11 * 7 + (10 - 8), // SQ_87
        11 * 8 + (10 - 8), // SQ_88
        11 * 9 + (10 - 8), // SQ_89
        11 * 1 + (10 - 9), // SQ_91
        11 * 2 + (10 - 9), // SQ_92
        11 * 3 + (10 - 9), // SQ_93
        11 * 4 + (10 - 9), // SQ_94
        11 * 5 + (10 - 9), // SQ_95
        11 * 6 + (10 - 9), // SQ_96
        11 * 7 + (10 - 9), // SQ_97
        11 * 8 + (10 - 9), // SQ_98
        11 * 9 + (10 - 9), // SQ_99
    ];

    TABLE[sq.inner() as usize]
}

/// マスを原作準拠の内部値に基づいて比較する。
pub fn naitou_square_cmp(lhs: Square, rhs: Square) -> std::cmp::Ordering {
    let lhs_value = naitou_square_to_value(lhs);
    let rhs_value = naitou_square_to_value(rhs);

    lhs_value.cmp(&rhs_value)
}

/// 2 つのマスの間のチェス盤距離を返す。
/// `sq1` が `None` の場合、10 筋 9 段目として扱う(原作準拠)。
pub const fn naitou_square_distance(sq1: Option<Square>, sq2: Square) -> u8 {
    match sq1 {
        Some(sq1) => sq1.distance(sq2),
        None => {
            let dx = (1 + COL_9.inner() - sq2.col().inner()) as u8;
            let dy = (ROW_9.inner() - sq2.row().inner()) as u8;
            if dx < dy {
                dy
            } else {
                dx
            }
        }
    }
}

/// 全マスを原作通りの順序で列挙する。(`SQ_91`, `SQ_81`, ..., `SQ_19` の順)
pub fn naitou_squares(
) -> impl Iterator<Item = Square> + DoubleEndedIterator + ExactSizeIterator + FusedIterator {
    // ExactSizeIterator にするため、配列をベタ書きする。
    #[rustfmt::skip]
    const SQS: [Square; 81] = [
        SQ_91, SQ_81, SQ_71, SQ_61, SQ_51, SQ_41, SQ_31, SQ_21, SQ_11,
        SQ_92, SQ_82, SQ_72, SQ_62, SQ_52, SQ_42, SQ_32, SQ_22, SQ_12,
        SQ_93, SQ_83, SQ_73, SQ_63, SQ_53, SQ_43, SQ_33, SQ_23, SQ_13,
        SQ_94, SQ_84, SQ_74, SQ_64, SQ_54, SQ_44, SQ_34, SQ_24, SQ_14,
        SQ_95, SQ_85, SQ_75, SQ_65, SQ_55, SQ_45, SQ_35, SQ_25, SQ_15,
        SQ_96, SQ_86, SQ_76, SQ_66, SQ_56, SQ_46, SQ_36, SQ_26, SQ_16,
        SQ_97, SQ_87, SQ_77, SQ_67, SQ_57, SQ_47, SQ_37, SQ_27, SQ_17,
        SQ_98, SQ_88, SQ_78, SQ_68, SQ_58, SQ_48, SQ_38, SQ_28, SQ_18,
        SQ_99, SQ_89, SQ_79, SQ_69, SQ_59, SQ_49, SQ_39, SQ_29, SQ_19,
    ];

    SQS.into_iter()
}

/// 原作準拠の内部値を駒に変換する。
pub fn naitou_piece_from_value(value: u8) -> Option<Piece> {
    match value {
        0 => Some(NO_PIECE),

        1 => Some(H_KING),
        2 => Some(H_ROOK),
        3 => Some(H_BISHOP),
        4 => Some(H_GOLD),
        5 => Some(H_SILVER),
        6 => Some(H_KNIGHT),
        7 => Some(H_LANCE),
        8 => Some(H_PAWN),
        9 => Some(H_DRAGON),
        10 => Some(H_HORSE),
        12 => Some(H_PRO_SILVER),
        13 => Some(H_PRO_KNIGHT),
        14 => Some(H_PRO_LANCE),
        15 => Some(H_PRO_PAWN),

        16 => Some(C_KING),
        17 => Some(C_ROOK),
        18 => Some(C_BISHOP),
        19 => Some(C_GOLD),
        20 => Some(C_SILVER),
        21 => Some(C_KNIGHT),
        22 => Some(C_LANCE),
        23 => Some(C_PAWN),
        24 => Some(C_DRAGON),
        25 => Some(C_HORSE),
        27 => Some(C_PRO_SILVER),
        28 => Some(C_PRO_KNIGHT),
        29 => Some(C_PRO_LANCE),
        30 => Some(C_PRO_PAWN),

        _ => None,
    }
}

/// 原作準拠の駒価値(テーブル A)を返す。
///
/// 用途:
///
/// * attacker 更新
/// * 捕獲する駒の価値算定
pub const fn naitou_piece_price_a(pk: PieceKind) -> u8 {
    const TABLE: [u8; 15] = [
        255, // NO_PIECE_KIND
        1,   // PAWN
        4,   // LANCE
        4,   // KNIGHT
        8,   // SILVER
        16,  // BISHOP
        17,  // ROOK
        8,   // GOLD
        40,  // KING
        2,   // PRO_PAWN
        5,   // PRO_LANCE
        6,   // PRO_KNIGHT
        8,   // PRO_SILVER
        20,  // HORSE
        22,  // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// 原作準拠の駒価値(テーブル B)を返す。
///
/// 用途:
///
/// * 駒得マス判定における HUM 駒、COM attacker の価値算定
pub const fn naitou_piece_price_b(pk: PieceKind) -> u8 {
    const TABLE: [u8; 15] = [
        255, // NO_PIECE_KIND
        1,   // PAWN
        4,   // LANCE
        4,   // KNIGHT
        8,   // SILVER
        16,  // BISHOP
        17,  // ROOK
        8,   // GOLD
        40,  // KING
        8,   // PRO_PAWN
        8,   // PRO_LANCE
        8,   // PRO_KNIGHT
        8,   // PRO_SILVER
        22,  // HORSE
        22,  // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// 原作準拠の駒価値(テーブル C)を返す。
///
/// 用途:
///
/// * 駒損マス判定における HUM attacker の価値算定
pub const fn naitou_piece_price_c(pk: PieceKind) -> u8 {
    const TABLE: [u8; 15] = [
        255, // NO_PIECE_KIND
        1,   // PAWN
        4,   // LANCE
        4,   // KNIGHT
        8,   // SILVER
        16,  // BISHOP
        17,  // ROOK
        8,   // GOLD
        40,  // KING
        2,   // PRO_PAWN
        8,   // PRO_LANCE
        8,   // PRO_KNIGHT
        8,   // PRO_SILVER
        22,  // HORSE
        22,  // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// 原作準拠の駒価値(テーブル D)を返す。
///
/// 用途:
///
/// * 駒損マス判定における COM 駒、COM attacker の価値算定
pub const fn naitou_piece_price_d(pk: PieceKind) -> u8 {
    const TABLE: [u8; 15] = [
        255, // NO_PIECE_KIND
        1,   // PAWN
        4,   // LANCE
        4,   // KNIGHT
        8,   // SILVER
        16,  // BISHOP
        17,  // ROOK
        8,   // GOLD
        40,  // KING
        1,   // PRO_PAWN
        4,   // PRO_LANCE
        4,   // PRO_KNIGHT
        8,   // PRO_SILVER
        20,  // HORSE
        22,  // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// 指定した局面、陣営について、マス `sq` に対する attacker を求める。
///
/// attacker とは、そのマスに利いている駒のうち最も価値の小さい駒種のこと(駒種の価値は原作準拠)。
/// 価値が同じなら原作準拠でマスの内部値が小さい方が優先される。
///
/// 影の利きは attacker に影響しない。
pub fn naitou_attacker(pos: &Position, us: Side, sq: Square) -> PieceKind {
    let them = us.inv();

    let mut current = (NO_PIECE_KIND, SQ_11);

    // sq に駒 (them, pk) を置いてみて、その利きに駒 (us, pk) があれば後者は sq に利いている。

    for pk in PieceKind::iter_piece() {
        let bb = bbs::effect(Piece::new(them, pk), sq, pos.bb_occupied()) & pos.bb_piece(us, pk);
        bb.for_each_square(|sq_attacker| {
            current = std::cmp::min_by_key(current, (pk, sq_attacker), |&(pk, sq)| {
                (naitou_piece_price_a(pk), naitou_square_to_value(sq))
            });
        });
    }

    current.0
}

/// 原作における COM 側の駒打ちの「移動元」(実際は駒種を表す)の内部値を返す。
/// `pk` は手駒となりうる駒種でなければならない。
///
/// 安い駒ほど値が小さくなる。
pub fn naitou_com_drop_src_value(pk: PieceKind) -> u8 {
    debug_assert!(pk.is_hand());

    match pk {
        PAWN => 201,
        LANCE => 202,
        KNIGHT => 203,
        SILVER => 204,
        GOLD => 205,
        BISHOP => 206,
        ROOK => 207,
        _ => unreachable!(),
    }
}
