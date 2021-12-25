//! 定跡関連。
//!
//! 定跡データは戦型別の「定跡分岐」と「定跡手順」からなる。
//!
//! 定跡分岐とは、「HUM 側のこの手にはこう応じるべし」という指示。
//! (正確には「このマスに HUM 側のこの駒があったら...」の意)
//! 具体的な応手指示と、戦型変更指示の 2 種がある。
//!
//! 定跡手順では、「この戦型ではこの手順で指し進めるべし」という指示。

use crate::bitop;
use crate::naitou::Handicap;
use crate::position::Position;
use crate::shogi::*;

/// 戦型。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Formation {
    Nakabisha,
    Sikenbisha,
    Kakugawari,
    Sujichigai,
    HumHishaochi,
    HumNimaiochi,
    ComHishaochi,
    ComNimaiochi,
    Nothing, // 定跡を抜けた後
}

impl Formation {
    /// 定跡を抜けた後かどうかを返す。
    pub const fn is_nothing(self) -> bool {
        matches!(self, Self::Nothing)
    }

    /// 指定された手合割に対応する初期戦型を返す。
    pub const fn from_handicap(handicap: Handicap) -> Self {
        match handicap {
            Handicap::HumSenteSikenbisha | Handicap::ComSenteSikenbisha => Self::Sikenbisha,
            Handicap::HumSenteNakabisha | Handicap::ComSenteNakabisha => Self::Nakabisha,
            Handicap::HumHishaochi => Self::HumHishaochi,
            Handicap::HumNimaiochi => Self::HumNimaiochi,
            Handicap::ComHishaochi => Self::ComHishaochi,
            Handicap::ComNimaiochi => Self::ComNimaiochi,
        }
    }

    /// 戦型に対応する定跡分岐を返す。`self` は `Nothing` であってはならない。
    const fn book_branch(self) -> &'static [BookBranchEntry] {
        match self {
            Self::Nakabisha => BOOK_BRANCH_NAKABISHA,
            Self::Sikenbisha => BOOK_BRANCH_SIKENBISHA,
            Self::Kakugawari => BOOK_BRANCH_KAKUGAWARI,
            Self::Sujichigai => BOOK_BRANCH_SUJICHIGAI,
            Self::HumHishaochi => BOOK_BRANCH_HUM_HISHAOCHI,
            Self::HumNimaiochi => BOOK_BRANCH_HUM_NIMAIOCHI,
            Self::ComHishaochi => BOOK_BRANCH_COM_HISHAOCHI,
            Self::ComNimaiochi => BOOK_BRANCH_COM_NIMAIOCHI,
            Self::Nothing => unreachable!(),
        }
    }

    /// 戦型に対応する定跡手順を返す。`self` は `Nothing` であってはならない。
    const fn book_moves(self) -> &'static [BookMovesEntry] {
        match self {
            Self::Nakabisha => BOOK_MOVES_NAKABISHA,
            Self::Sikenbisha => BOOK_MOVES_SIKENBISHA,
            Self::Kakugawari => BOOK_MOVES_KAKUGAWARI,
            Self::Sujichigai => BOOK_MOVES_SUJICHIGAI,
            Self::HumHishaochi => BOOK_MOVES_HUM_HISHAOCHI,
            Self::HumNimaiochi => BOOK_MOVES_HUM_NIMAIOCHI,
            Self::ComHishaochi => BOOK_MOVES_COM_HISHAOCHI,
            Self::ComNimaiochi => BOOK_MOVES_COM_NIMAIOCHI,
            Self::Nothing => unreachable!(),
        }
    }
}

impl std::fmt::Display for Formation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Nakabisha => f.write_str("中飛車"),
            Self::Sikenbisha => f.write_str("四間飛車"),
            Self::Kakugawari => f.write_str("角換わり"),
            Self::Sujichigai => f.write_str("筋違い角"),
            Self::HumHishaochi => f.write_str("HUM 飛車落ち"),
            Self::HumNimaiochi => f.write_str("HUM 二枚落ち"),
            Self::ComHishaochi => f.write_str("COM 飛車落ち"),
            Self::ComNimaiochi => f.write_str("COM 二枚落ち"),
            Self::Nothing => f.write_str("(なし)"),
        }
    }
}

/// 定跡分岐エントリ。
#[derive(Debug)]
enum BookBranchEntry {
    Move(BookBranchMoveEntry),
    ChangeFormation(BookBranchChangeFormationEntry),
}

impl BookBranchEntry {
    const fn new_move(sq: Square, pk: PieceKind, src: Square, dst: Square) -> Self {
        Self::Move(BookBranchMoveEntry { sq, pk, src, dst })
    }

    const fn new_change_formation(
        sq: Square,
        pk: PieceKind,
        formation: Formation,
        ply: u8,
    ) -> Self {
        Self::ChangeFormation(BookBranchChangeFormationEntry {
            sq,
            pk,
            formation,
            ply,
        })
    }
}

/// 定跡分岐 応手指示エントリ。
///
/// `sq` に HUM 駒 `pk` があったら walk 手 (src, dst) で応じる(常に不成)。
#[derive(Debug)]
struct BookBranchMoveEntry {
    sq: Square,
    pk: PieceKind,
    src: Square,
    dst: Square,
}

impl BookBranchMoveEntry {
    /// 渡された局面がこの応手指示エントリの対象となるかどうかを返す。
    fn matches(&self, pos: &Position) -> bool {
        pos.board()[self.sq] == Piece::new(HUM, self.pk)
    }
}

/// 定跡分岐 戦型変更指示エントリ。
///
/// 手数(進行度管理用)が `ply` 以内で、`sq` に HUM 駒 `pk` があったら戦型を `formation` に変更。
#[derive(Debug)]
struct BookBranchChangeFormationEntry {
    sq: Square,
    pk: PieceKind,
    formation: Formation,
    ply: u8,
}

impl BookBranchChangeFormationEntry {
    /// 渡された局面および `progress_ply` がこの戦型変更指示エントリの対象となるかどうかを返す。
    fn matches(&self, pos: &Position, progress_ply: u8) -> bool {
        pos.board()[self.sq] == Piece::new(HUM, self.pk) && progress_ply <= self.ply
    }
}

/// 定跡手順エントリ。
///
/// walk 手 (src, dst) (常に不成)。
#[derive(Debug)]
struct BookMovesEntry {
    src: Square,
    dst: Square,
}

impl BookMovesEntry {
    const fn new(src: Square, dst: Square) -> Self {
        Self { src, dst }
    }
}

/// 定跡処理用の管理データ。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BookState {
    formation: Formation,    // 現在の戦型
    mask_unused_branch: u32, // ビット i が 1 ならば定跡分岐 i 個目が未使用
    mask_unused_moves: u32,  // ビット i が 1 ならば定跡手順 i 個目が未使用
}

impl BookState {
    /// 初期戦型を指定して `BookState` を作る。`formation` は `Nothing` であってはならない。
    pub fn new(formation: Formation) -> Self {
        debug_assert_ne!(formation, Formation::Nothing);

        let mut this = Self {
            formation: Formation::Nothing,
            mask_unused_branch: 0,
            mask_unused_moves: 0,
        };
        this.change_formation(formation);

        this
    }

    /// 現在の戦型を返す。
    pub fn formation(self) -> Formation {
        self.formation
    }

    /// 戦型を変更し、定跡分岐/定跡手順の使用状況を再初期化する。
    fn change_formation(&mut self, formation: Formation) {
        self.formation = formation;

        self.mask_unused_branch = (1 << formation.book_branch().len()) - 1;
        self.mask_unused_moves = (1 << formation.book_moves().len()) - 1;
    }

    /// 次の定跡手を得る。現在の戦型が `Formation::Nothing` であってはならない。
    /// 定跡を抜けた(戦型が `Formation::Nothing` になった)場合、`None` を返す。
    ///
    /// 合法性や駒損チェックは行わないので、呼び出し側で適切に処理すること。
    pub fn next_move(&mut self, pos: &Position, progress_ply: u8) -> Option<Move> {
        // COM 先手の場合、初手のみ定跡手を指しても unused フラグが下りない(原作通り)。
        // 原作では使用済みであることを示すために指した時点での `progress_ply` を記録するが、
        // COM 先手のときの初手ではその値が 0 になるのが原因。

        debug_assert_ne!(self.formation, Formation::Nothing);

        // 定跡分岐の処理。
        'book_branch: loop {
            // 未使用の定跡分岐を順に試す。
            for i in bitop::iter_ones_u32(self.mask_unused_branch) {
                let e = &self.formation.book_branch()[i as usize];
                match e {
                    BookBranchEntry::Move(bra_mv) => {
                        // 局面が応手指示の対象ならば、その応手を返す。
                        if bra_mv.matches(pos) {
                            if progress_ply != 0 {
                                self.mask_unused_branch &= !(1 << i);
                            }
                            return Some(Move::new_walk(bra_mv.src, bra_mv.dst));
                        }
                    }
                    BookBranchEntry::ChangeFormation(bra_change) => {
                        // 局面および progress_ply が戦型変更指示の対象ならば、
                        // 戦型変更して定跡分岐処理を最初からやり直す。
                        if bra_change.matches(pos, progress_ply) {
                            self.change_formation(bra_change.formation);
                            continue 'book_branch;
                        }
                    }
                }
            }
            // どの応手指示も採用されなかった場合、定跡手順の処理へ。
            break;
        }

        // 定跡手順の処理。未使用の定跡手があればそれを返す。
        if self.mask_unused_moves != 0 {
            let i = bitop::lsb_u32(self.mask_unused_moves);
            let e = &self.formation.book_moves()[i as usize];
            if progress_ply != 0 {
                self.mask_unused_moves &= !(1 << i);
            }
            return Some(Move::new_walk(e.src, e.dst));
        }

        // 全ての定跡手が使用済みならば定跡を抜ける。
        self.formation = Formation::Nothing;

        None
    }
}

/// 平手 中飛車 定跡分岐。
const BOOK_BRANCH_NAKABISHA: &[BookBranchEntry] = &[
    BookBranchEntry::new_change_formation(SQ_22, BISHOP, Formation::Kakugawari, 5),
    BookBranchEntry::new_change_formation(SQ_22, HORSE, Formation::Kakugawari, 5),
    BookBranchEntry::new_move(SQ_55, BISHOP, SQ_53, SQ_54),
    BookBranchEntry::new_move(SQ_46, BISHOP, SQ_44, SQ_45),
    BookBranchEntry::new_move(SQ_46, SILVER, SQ_44, SQ_45),
    BookBranchEntry::new_move(SQ_26, SILVER, SQ_41, SQ_32),
    BookBranchEntry::new_move(SQ_46, PAWN, SQ_22, SQ_33),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_25, PAWN, SQ_22, SQ_33),
    BookBranchEntry::new_move(SQ_35, SILVER, SQ_44, SQ_45),
];

/// 平手 四間飛車 定跡分岐。
const BOOK_BRANCH_SIKENBISHA: &[BookBranchEntry] = &[
    BookBranchEntry::new_change_formation(SQ_22, BISHOP, Formation::Kakugawari, 5),
    BookBranchEntry::new_change_formation(SQ_22, HORSE, Formation::Kakugawari, 5),
    BookBranchEntry::new_move(SQ_55, BISHOP, SQ_53, SQ_54),
    BookBranchEntry::new_move(SQ_46, BISHOP, SQ_44, SQ_45),
    BookBranchEntry::new_move(SQ_46, SILVER, SQ_44, SQ_45),
    BookBranchEntry::new_move(SQ_26, SILVER, SQ_42, SQ_32),
    BookBranchEntry::new_move(SQ_46, PAWN, SQ_22, SQ_33),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_25, PAWN, SQ_22, SQ_33),
    BookBranchEntry::new_move(SQ_35, SILVER, SQ_44, SQ_45),
];

/// 平手 角換わり 定跡分岐。
const BOOK_BRANCH_KAKUGAWARI: &[BookBranchEntry] = &[
    BookBranchEntry::new_change_formation(SQ_45, BISHOP, Formation::Sujichigai, 5),
    BookBranchEntry::new_change_formation(SQ_56, BISHOP, Formation::Sujichigai, 5),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
];

/// 平手 筋違い角 定跡分岐。
const BOOK_BRANCH_SUJICHIGAI: &[BookBranchEntry] = &[
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_16, PAWN, SQ_13, SQ_14),
];

/// HUM 飛車落ち 定跡分岐。
const BOOK_BRANCH_HUM_HISHAOCHI: &[BookBranchEntry] = &[
    BookBranchEntry::new_move(SQ_16, PAWN, SQ_13, SQ_14),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_22, BISHOP, SQ_31, SQ_22),
    BookBranchEntry::new_move(SQ_22, HORSE, SQ_31, SQ_22),
];

/// HUM 二枚落ち 定跡分岐。
const BOOK_BRANCH_HUM_NIMAIOCHI: &[BookBranchEntry] =
    &[BookBranchEntry::new_move(SQ_56, PAWN, SQ_53, SQ_54)];

/// COM 飛車落ち 定跡分岐。
const BOOK_BRANCH_COM_HISHAOCHI: &[BookBranchEntry] = &[
    BookBranchEntry::new_move(SQ_25, PAWN, SQ_22, SQ_33),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_16, PAWN, SQ_13, SQ_14),
];

/// COM 二枚落ち 定跡分岐。
const BOOK_BRANCH_COM_NIMAIOCHI: &[BookBranchEntry] = &[
    BookBranchEntry::new_move(SQ_16, PAWN, SQ_13, SQ_14),
    BookBranchEntry::new_move(SQ_96, PAWN, SQ_93, SQ_94),
    BookBranchEntry::new_move(SQ_56, PAWN, SQ_53, SQ_54),
    BookBranchEntry::new_move(SQ_35, PAWN, SQ_31, SQ_22),
];

/// 平手 中飛車 定跡手順。
const BOOK_MOVES_NAKABISHA: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_43, SQ_44),
    BookMovesEntry::new(SQ_31, SQ_42),
    BookMovesEntry::new(SQ_82, SQ_52),
    BookMovesEntry::new(SQ_42, SQ_43),
    BookMovesEntry::new(SQ_51, SQ_62),
    BookMovesEntry::new(SQ_62, SQ_72),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_22, SQ_33),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_61, SQ_62),
    BookMovesEntry::new(SQ_41, SQ_42),
    BookMovesEntry::new(SQ_42, SQ_53),
    BookMovesEntry::new(SQ_52, SQ_22),
    BookMovesEntry::new(SQ_23, SQ_24),
    BookMovesEntry::new(SQ_24, SQ_25),
    BookMovesEntry::new(SQ_44, SQ_45),
];

/// 平手 四間飛車 定跡手順。
const BOOK_MOVES_SIKENBISHA: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_43, SQ_44),
    BookMovesEntry::new(SQ_31, SQ_32),
    BookMovesEntry::new(SQ_82, SQ_42),
    BookMovesEntry::new(SQ_32, SQ_43),
    BookMovesEntry::new(SQ_51, SQ_62),
    BookMovesEntry::new(SQ_62, SQ_72),
    BookMovesEntry::new(SQ_72, SQ_82),
    BookMovesEntry::new(SQ_71, SQ_72),
    BookMovesEntry::new(SQ_41, SQ_52),
    BookMovesEntry::new(SQ_22, SQ_33),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_52, SQ_63),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_42, SQ_41),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_44, SQ_45),
];

/// 平手 角換わり 定跡手順。
const BOOK_MOVES_KAKUGAWARI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_31, SQ_22),
    BookMovesEntry::new(SQ_22, SQ_33),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_83, SQ_84),
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_84, SQ_85),
    BookMovesEntry::new(SQ_61, SQ_52),
    BookMovesEntry::new(SQ_51, SQ_41),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_41, SQ_31),
    BookMovesEntry::new(SQ_31, SQ_22),
    BookMovesEntry::new(SQ_43, SQ_44),
    BookMovesEntry::new(SQ_52, SQ_43),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_81, SQ_73),
    BookMovesEntry::new(SQ_64, SQ_65),
    BookMovesEntry::new(SQ_63, SQ_54),
];

/// 平手 筋違い角 定跡手順。
const BOOK_MOVES_SUJICHIGAI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_31, SQ_22),
    BookMovesEntry::new(SQ_61, SQ_52),
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_22, SQ_33),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_83, SQ_84),
    BookMovesEntry::new(SQ_84, SQ_85),
    BookMovesEntry::new(SQ_51, SQ_41),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_81, SQ_73),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_13, SQ_14),
    BookMovesEntry::new(SQ_33, SQ_44),
    BookMovesEntry::new(SQ_64, SQ_65),
];

/// HUM 飛車落ち 定跡手順。
const BOOK_MOVES_HUM_HISHAOCHI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_83, SQ_84),
    BookMovesEntry::new(SQ_84, SQ_85),
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_61, SQ_52),
    BookMovesEntry::new(SQ_51, SQ_41),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_31, SQ_42),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_81, SQ_73),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_13, SQ_14),
    BookMovesEntry::new(SQ_22, SQ_33),
    BookMovesEntry::new(SQ_64, SQ_65),
];

/// HUM 二枚落ち 定跡手順。
const BOOK_MOVES_HUM_NIMAIOCHI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_64, SQ_65),
    BookMovesEntry::new(SQ_82, SQ_62),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_74, SQ_75),
    BookMovesEntry::new(SQ_71, SQ_72),
    BookMovesEntry::new(SQ_72, SQ_73),
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_61, SQ_52),
    BookMovesEntry::new(SQ_51, SQ_41),
    BookMovesEntry::new(SQ_31, SQ_42),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_81, SQ_73),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_13, SQ_14),
    BookMovesEntry::new(SQ_62, SQ_61),
    BookMovesEntry::new(SQ_75, SQ_76),
];

/// COM 飛車落ち 定跡手順。
const BOOK_MOVES_COM_HISHAOCHI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_33, SQ_34),
    BookMovesEntry::new(SQ_43, SQ_44),
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_31, SQ_42),
    BookMovesEntry::new(SQ_42, SQ_43),
    BookMovesEntry::new(SQ_51, SQ_62),
    BookMovesEntry::new(SQ_62, SQ_72),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_13, SQ_14),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_61, SQ_62),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_22, SQ_33),
];

/// COM 二枚落ち 定跡手順。
const BOOK_MOVES_COM_NIMAIOCHI: &[BookMovesEntry] = &[
    BookMovesEntry::new(SQ_41, SQ_32),
    BookMovesEntry::new(SQ_71, SQ_62),
    BookMovesEntry::new(SQ_53, SQ_54),
    BookMovesEntry::new(SQ_62, SQ_53),
    BookMovesEntry::new(SQ_61, SQ_62),
    BookMovesEntry::new(SQ_63, SQ_64),
    BookMovesEntry::new(SQ_62, SQ_63),
    BookMovesEntry::new(SQ_73, SQ_74),
    BookMovesEntry::new(SQ_51, SQ_62),
    BookMovesEntry::new(SQ_13, SQ_14),
    BookMovesEntry::new(SQ_93, SQ_94),
    BookMovesEntry::new(SQ_81, SQ_73),
    BookMovesEntry::new(SQ_31, SQ_42),
    BookMovesEntry::new(SQ_64, SQ_65),
];
