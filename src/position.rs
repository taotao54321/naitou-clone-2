// 利き情報の差分更新については、やねうら王公式サイトの解説を参照:
//
// * [Long Effect Library 完全解説 その1](https://yaneuraou.yaneu.com/2016/01/23/long-effect-library-%e5%ae%8c%e5%85%a8%e8%a7%a3%e8%aa%ac-%e3%81%9d%e3%81%ae1/)
// * [Long Effect Library 完全解説 その2](https://yaneuraou.yaneu.com/2016/01/24/long-effect-library-%e5%ae%8c%e5%85%a8%e8%a7%a3%e8%aa%ac-%e3%81%9d%e3%81%ae2/)
// * [Long Effect Library 完全解説 その3](https://yaneuraou.yaneu.com/2016/01/25/long-effect-library-%e5%ae%8c%e5%85%a8%e8%a7%a3%e8%aa%ac-%e3%81%9d%e3%81%ae3/)
// * [Long Effect Library 完全解説 その4](https://yaneuraou.yaneu.com/2016/01/29/long-effect-library-%e5%ae%8c%e5%85%a8%e8%a7%a3%e8%aa%ac-%e3%81%9d%e3%81%ae4/)

use crate::bbs;
use crate::bitboard::Bitboard;
use crate::effect::*;
use crate::movegen::position_is_checkmated;
use crate::myarray::*;
use crate::mynum::WrappingAddAssign as _;
use crate::shogi::*;

type BbOccSide = MyArray1<Bitboard, Side, 2>;
type BbPk = MyArray1<Bitboard, PieceKind, 15>;
type KingSq = MyArray1<Square, Side, 2>;
type EffectCountBoards = MyArray1<EffectCountBoard, Side, 2>;

/// 局面。
#[derive(Clone, Debug)]
pub struct Position {
    // 一応アラインメントを要求するものを先に並べたが、
    // Rust はデフォルトでは構造体のメモリレイアウトは未定義。
    // (https://doc.rust-lang.org/stable/reference/types/struct.html)
    // 多分よきに計らってくれるはず?
    bb_occ: Bitboard,       // 陣営を区別しない occupied bitboard
    bb_occ_side: BbOccSide, // 陣営を区別する occupied bitboard
    bb_pk: BbPk,            // 駒種ごとの bitboard (陣営の区別なし)

    effect_counts: EffectCountBoards, // 各陣営の盤面上の各マスの利き数
    ranged_effects: RangedEffectBoard, // 盤面上の各マスにおける両陣営の遠隔利き

    board: Board,
    hands: Hands,
    side_to_move: Side,
    ply: u32, // 常に 1 から始まるものとする。

    king_sq: KingSq, // 各陣営の玉位置

    com_nonking_count: u32, // COM 側の玉以外の駒数(盤上の駒と手駒の合計)
}

impl Position {
    /// 手番、盤面、両陣営の手駒を指定して局面を作る。
    /// 合法性チェックは一切行わない。
    pub fn new(side_to_move: Side, board: Board, hands: Hands) -> Self {
        // occupied bitboard, 駒種ごとの bitboard, 玉位置を求める。

        let mut bb_occ = Bitboard::zero();
        let mut bb_occ_side = BbOccSide::default();
        let mut bb_pk = BbPk::default();

        // 両陣営とも玉は存在すると仮定している。
        let mut king_sq = KingSq::from([SQ_11; 2]);

        for sq in Square::iter() {
            let pc = board[sq];
            if !pc.is_piece() {
                continue;
            }

            bb_occ |= Bitboard::from(sq);
            bb_occ_side[pc.side()] |= Bitboard::from(sq);
            bb_pk[pc.kind()] |= Bitboard::from(sq);

            if pc.kind() == KING {
                king_sq[pc.side()] = sq;
            }
        }

        let effect_counts =
            EffectCountBoards::from([EffectCountBoard::empty(), EffectCountBoard::empty()]);
        let ranged_effects = RangedEffectBoard::empty();

        let mut com_nonking_count = bb_occ_side[COM].count_ones() - 1;
        for pk in PieceKind::iter_hand() {
            com_nonking_count += hands[COM][pk];
        }

        let mut this = Self {
            bb_occ,
            bb_occ_side,
            bb_pk,

            effect_counts,
            ranged_effects,

            board,
            hands,
            side_to_move,
            ply: 1,

            king_sq,

            com_nonking_count,
        };

        let (effect_counts, ranged_effects) = calc_effect(&this);
        this.effect_counts = effect_counts;
        this.ranged_effects = ranged_effects;

        this
    }

    /// 手数を返す。
    pub fn ply(&self) -> u32 {
        self.ply
    }

    /// 手番を返す。
    pub fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    /// 盤面への参照を返す。
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// 両陣営の手駒への参照を返す。
    pub fn hands(&self) -> &Hands {
        &self.hands
    }

    /// 指定した陣営の手駒への参照を返す。
    pub fn hand(&self, side: Side) -> &Hand {
        &self.hands[side]
    }

    /// 陣営を区別しない occupied bitboard を返す。
    pub fn bb_occupied(&self) -> Bitboard {
        self.bb_occ
    }

    /// 指定した陣営の occupied bitboard を返す。
    pub fn bb_occupied_side(&self, side: Side) -> Bitboard {
        self.bb_occ_side[side]
    }

    /// 指定した駒種の bitboard (陣営を区別しない)を返す。
    /// `pk` は実際の駒でなければならない。
    pub fn bb_piece_kind(&self, pk: PieceKind) -> Bitboard {
        debug_assert!(pk.is_piece());

        self.bb_pk[pk]
    }

    /// 指定した陣営、駒種の bitboard を返す。
    /// `pk` は実際の駒でなければならない。
    pub fn bb_piece(&self, side: Side, pk: PieceKind) -> Bitboard {
        self.bb_occupied_side(side) & self.bb_piece_kind(pk)
    }

    /// 空白マスのみが 1 になっている bitboard を返す。
    pub fn bb_blank(&self) -> Bitboard {
        self.bb_occ ^ Bitboard::all()
    }

    /// 指定した陣営の `EffectCountBoard` への参照を返す。
    pub fn effect_count_board(&self, side: Side) -> &EffectCountBoard {
        &self.effect_counts[side]
    }

    /// 指定した陣営の玉位置を返す。
    pub fn king_square(&self, side: Side) -> Square {
        self.king_sq[side]
    }

    /// COM 側の玉以外の駒数(盤上の駒と手駒の合計)を返す。
    /// 全駒勝利手順を求める際の枝刈りに使う。
    pub fn com_nonking_count(&self) -> u32 {
        self.com_nonking_count
    }

    /// 指し手で局面を進め、`UndoableMove` を返す。
    ///
    /// `mv` は少なくとも疑似合法手であり、かつ玉を取る手ではないと仮定している。
    pub fn do_move(&mut self, mv: Move) -> UndoableMove {
        debug_assert!(mv.is_valid());

        /* for debug
        eprintln!("--- do_move() start ---");
        eprintln!("{}", self);
        eprintln!("{}", self.effect_counts[HUM]);
        eprintln!("{}", self.effect_counts[COM]);
        eprintln!("{}", self.ranged_effects);
        */

        let umv = if mv.is_drop() {
            self.do_move_drop(mv)
        } else {
            self.do_move_walk(mv)
        };

        self.side_to_move = self.side_to_move.inv();
        self.ply += 1;

        /* for debug
        {
            let (effect_counts, ranged_effects) = calc_effect(self);
            let ng_count_hum = self.effect_counts[HUM] != effect_counts[HUM];
            let ng_count_com = self.effect_counts[COM] != effect_counts[COM];
            let ng_range = self.ranged_effects != ranged_effects;
            let ng = ng_count_hum || ng_count_com || ng_range;
            if ng {
                eprintln!("[do_move() failed]");
                eprintln!("{}", self);
                eprintln!("指し手: {}", umv);
            }
            if ng_count_hum {
                eprintln!("--- HUM counts ---");
                eprintln!("{}", self.effect_counts[HUM]);
                eprintln!("{}", effect_counts[HUM]);
            }
            if ng_count_com {
                eprintln!("--- COM counts ---");
                eprintln!("{}", self.effect_counts[COM]);
                eprintln!("{}", effect_counts[COM]);
            }
            if ng_range {
                eprintln!("--- ranged ---");
                eprintln!("{}", self.ranged_effects);
                eprintln!("{}", ranged_effects);
            }
            assert!(!ng);
        }
        */

        umv
    }

    /// 盤上の駒を動かす指し手で局面を進め、`UndoableMove` を返す。
    fn do_move_walk(&mut self, mv: Move) -> UndoableMove {
        let us = self.side_to_move;
        let src = mv.src();
        let dst = mv.dst();
        let promo = mv.is_promotion();

        let pc_src = self.board[src];
        // 移動元の駒は自駒でなければならない。
        debug_assert!(pc_src.is_piece());
        debug_assert_eq!(pc_src.side(), us);
        // 成りの場合、移動元の駒は成れる駒でなければならない。
        debug_assert!(!promo || pc_src.is_promotable());

        let pc_captured = self.board[dst];
        // 移動先は自駒であってはならない。
        debug_assert!(pc_captured == NO_PIECE || pc_captured.side() != us);
        // 玉を取る手は不可。
        debug_assert_ne!(pc_captured.kind(), KING);

        // 移動後の駒を求める(成りの場合は成るということ)。
        let pc_dst = if promo { pc_src.to_promoted() } else { pc_src };

        // 駒取りか?
        if pc_captured == NO_PIECE {
            // 駒取りでない場合、単に利き情報を更新。
            // 差分計算のために着手前の局面が必要なので、まだ盤面は動かさない。
            self.update_effect_by_noncapture(src, dst, pc_src, pc_dst);
        } else {
            // 駒取りの場合、まず利き情報を更新。
            // これも盤面を動かす前に行う。
            self.update_effect_by_capture(src, dst, pc_src, pc_dst, pc_captured);

            // 捕獲した駒を us 側の手駒に加える。
            let pr_captured = pc_captured.to_raw_kind();
            self.hands[us][pr_captured] += 1;

            // 捕獲した駒を盤上から除去する。
            self.remove_piece(dst);

            // COM 側の駒数を更新。
            if us == HUM {
                self.com_nonking_count -= 1;
            } else {
                self.com_nonking_count += 1;
            }
        }

        // 移動元から駒を除去し、移動先に移動後の駒を置く。
        self.remove_piece(src);
        self.put_piece(dst, pc_dst);

        // 玉を動かした場合、玉位置を更新。
        if pc_src.kind() == KING {
            self.king_sq[us] = dst;
        }

        UndoableMove::from_move_walk(mv, pc_src, pc_captured)
    }

    /// 盤上の駒を動かす駒取りの指し手による利き情報更新処理。
    /// 呼び出し時点ではまだ着手前の局面になっている(手番、盤面とも)。
    fn update_effect_by_capture(
        &mut self,
        src: Square,
        dst: Square,
        pc_src: Piece,
        pc_dst: Piece,
        pc_captured: Piece,
    ) {
        // 盤上の駒を動かす指し手で、かつ駒取りの場合、
        //
        // * 移動元の駒の近接利きが除去される。
        // * 移動先の駒の近接利きが発生する。
        // * 捕獲された駒の近接利きが除去される。
        //
        // * 移動元の駒の影の利きが除去される。
        // * 捕獲された駒の影の利きが除去される。
        //
        // * 移動元の駒の遠隔利きが除去される。
        // * 移動元のマスに存在していた遠隔利きが開放される。
        //
        // * 移動先の駒の影の利きが発生する。
        //
        // * 移動先の駒の遠隔利きが発生する。
        // * 捕獲された駒の遠隔利きが除去される。
        //
        // 処理順に注意。影の利きを求める際、適切な時点での遠隔利きを選ばないといけない。
        //
        // 移動先には元々駒があったので、移動先での遮断処理は起こらないことに注意。

        let us = self.side_to_move; // 指した側
        let them = us.inv();

        // 移動元の駒の近接利き除去と、移動先の駒の近接利き発生。
        // 相殺する部分は利き数が変化しないのでマスクすると無駄がない。
        {
            let mut bb_dec = bbs::effect_melee(pc_src, src); // 利き数を減らすマスたち
            let mut bb_inc = bbs::effect_melee(pc_dst, dst); // 利き数を増やすマスたち

            let bb_intersect = bb_dec & bb_inc;
            bb_dec ^= bb_intersect;
            bb_inc ^= bb_intersect;

            bb_dec.for_each_square(|sq| {
                self.effect_counts[us][sq] -= 1;
            });
            bb_inc.for_each_square(|sq| {
                self.effect_counts[us][sq] += 1;
            });
        }

        // 捕獲された駒の近接利き除去。
        bbs::effect_melee(pc_captured, dst).for_each_square(|sq| {
            self.effect_counts[them][sq] -= 1;
        });

        // 移動元の駒の影の利き除去。
        {
            let src_ww = SquareWithWall::from(src);
            let dirs_pc_src = DirectionSet::from_piece_supported(pc_src);
            let dirs_support_src = dirs_pc_src & self.ranged_effects[src].get(us);
            dirs_support_src.for_each(|dir| {
                let sq_ww = src_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] -= 1;
                }
            });
        }

        // 捕獲された駒の影の利き除去。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_captured = DirectionSet::from_piece_supported(pc_captured);
            let dirs_support_captured = dirs_pc_captured & self.ranged_effects[dst].get(them);
            dirs_support_captured.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[them][Square::from(sq_ww)] -= 1;
                }
            });
        }

        // 移動元における遠隔利き更新。
        // 8 方向への移動の場合、移動と逆方向の利きは先後とも更新しなくてよい。
        {
            let mv_dirs = DirectionSet::from_squares(src, dst);
            let dsp_mask = if mv_dirs.is_empty() {
                // 8 方向への移動でないのは桂のみ。
                DirectionSetPair::all()
            } else {
                let mv_dirs_inv = DirectionSet::from(mv_dirs.get_least().inv());
                !DirectionSetPair::new(mv_dirs_inv, mv_dirs_inv)
            };

            // 移動元の駒が持っていた遠隔利き(除去される)。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_src) & dsp_mask;

            // 移動元の既存の遠隔利き(開放される)。
            let dsp_others = self.ranged_effects[src] & dsp_mask;

            // hack: この時点では捕獲される駒がまだ残っているため、
            // 例えば金が取られる場合、そこへ飛車の利きが開放されると、偽の影の利きが生じてしまう。
            // そこで、捕獲される駒を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_captured_orig = std::mem::replace(&mut self.board[dst], H_KNIGHT);
            self.update_effect_ranged(src, dsp_us, dsp_others, false);
            self.board[dst] = pc_captured_orig;
        }

        // 移動先の駒の影の利き発生。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
            let dirs_support_dst = dirs_pc_dst & self.ranged_effects[dst].get(us);
            dirs_support_dst.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] += 1;
                }
            });
        }

        // 移動先における遠隔利き更新。ここでは遮断処理は起こらない。
        {
            // 移動先の駒により発生する遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_dst);

            // 捕獲された駒が持っていた遠隔利き(除去される)。
            let dsp_others = DirectionSetPair::from_piece_ranged(pc_captured);

            // hack:
            // この時点では移動元の駒がまだ残っているため、
            // 例えば角を移動した際に移動元に対して偽の影の利きが生じてしまう。
            // そこで、移動元を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_src_orig = std::mem::replace(&mut self.board[src], H_KNIGHT);
            self.update_effect_ranged(dst, dsp_us, dsp_others, true);
            self.board[src] = pc_src_orig;
        }
    }

    /// 盤上の駒を動かす駒取りでない指し手による利き情報更新処理。
    /// 呼び出し時点ではまだ着手前の局面になっている(手番、盤面とも)。
    fn update_effect_by_noncapture(
        &mut self,
        src: Square,
        dst: Square,
        pc_src: Piece,
        pc_dst: Piece,
    ) {
        // 盤上の駒を動かす指し手で、かつ駒取りでない場合、
        //
        // * 移動元の駒の近接利きが除去される。
        // * 移動先の駒の近接利きが発生する。
        //
        // * 移動元の駒の影の利きが除去される。
        //
        // * 移動元の駒の遠隔利きが除去される。
        // * 移動元のマスに存在していた遠隔利きが開放される。
        //
        // * 移動先の駒の影の利きが発生する。
        //
        // * 移動先の駒の遠隔利きが発生する。
        // * 移動先のマスに存在していた遠隔利きが遮断される。

        let us = self.side_to_move; // 指した側

        // 移動元の駒の近接利き除去と、移動先の駒の近接利き発生。
        // 相殺する部分は利き数が変化しないのでマスクすると無駄がない。
        {
            let mut bb_dec = bbs::effect_melee(pc_src, src); // 利き数を減らすマスたち
            let mut bb_inc = bbs::effect_melee(pc_dst, dst); // 利き数を増やすマスたち

            let bb_intersect = bb_dec & bb_inc;
            bb_dec ^= bb_intersect;
            bb_inc ^= bb_intersect;

            bb_dec.for_each_square(|sq| {
                self.effect_counts[us][sq] -= 1;
            });
            bb_inc.for_each_square(|sq| {
                self.effect_counts[us][sq] += 1;
            });
        }

        // 移動元の駒の影の利き除去。
        {
            let src_ww = SquareWithWall::from(src);
            let dirs_pc_src = DirectionSet::from_piece_supported(pc_src);
            let dirs_support_src = dirs_pc_src & self.ranged_effects[src].get(us);
            dirs_support_src.for_each(|dir| {
                let sq_ww = src_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] -= 1;
                }
            });
        }

        // 移動元における遠隔利き更新。
        // 8 方向への移動の場合、移動と逆方向の利きは先後とも更新しなくてよい。
        {
            let mv_dirs = DirectionSet::from_squares(src, dst);
            let dsp_mask = if mv_dirs.is_empty() {
                // 8 方向への移動でないのは桂のみ。
                DirectionSetPair::all()
            } else {
                let mv_dirs_inv = DirectionSet::from(mv_dirs.get_least().inv());
                !DirectionSetPair::new(mv_dirs_inv, mv_dirs_inv)
            };

            // 移動元の駒が持っていた遠隔利き(除去される)。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_src) & dsp_mask;

            // 移動元の既存の遠隔利き(開放される)。
            let dsp_others = self.ranged_effects[src] & dsp_mask;

            self.update_effect_ranged(src, dsp_us, dsp_others, false);
        }

        // 移動先の駒の影の利き発生。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
            let dirs_support_dst = dirs_pc_dst & self.ranged_effects[dst].get(us);
            dirs_support_dst.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] += 1;
                }
            });
        }

        // 移動先における遠隔利き更新。
        {
            // 移動先の駒により発生する遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_dst);

            // 移動先の駒により遮断される遠隔利き。
            let dsp_others = self.ranged_effects[dst];

            // hack:
            // この時点では移動元の駒がまだ残っているため、
            // 例えば角を移動した際に移動元に対して偽の影の利きが生じてしまう。
            // そこで、移動元を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_src_orig = std::mem::replace(&mut self.board[src], H_KNIGHT);
            self.update_effect_ranged(dst, dsp_us, dsp_others, true);
            self.board[src] = pc_src_orig;
        }
    }

    /// 駒打ちの指し手で局面を進め、`UndoableMove` を返す。
    fn do_move_drop(&mut self, mv: Move) -> UndoableMove {
        let us = self.side_to_move;
        let pk = mv.dropped_piece_kind();
        let dst = mv.dst();

        // 移動先は空白でなければならない。
        debug_assert_eq!(self.board[dst], NO_PIECE);
        // 打つ駒種が手駒になければならない。
        debug_assert!(self.hands[us][pk] > 0);

        // 打つ駒種を手駒から減らす。
        self.hands[us][pk] -= 1;

        // pk を自駒として移動先に置く。
        self.put_piece(dst, Piece::new(us, pk));

        // drop では玉位置の更新は起こらない。

        // 利き情報を更新。
        self.update_effect_by_drop(pk, dst);

        UndoableMove::from_move_drop(mv)
    }

    /// 駒打ちの指し手による利き情報更新処理。
    /// 呼び出し時点ではまだ指した側の手番になっている。
    fn update_effect_by_drop(&mut self, pk: PieceKind, dst: Square) {
        // 駒打ちの場合、
        //
        // * 打った駒の近接利きが発生する。
        // * 打った駒の影の利きが発生する。
        // * 打った駒の遠隔利きが発生する。
        // * 打ったマスに存在していた遠隔利きが遮断される。

        let us = self.side_to_move;
        let pc = Piece::new(us, pk);

        // 打った駒の近接利きを利き数に加える。
        bbs::effect_melee(pc, dst).for_each_square(|sq| {
            self.effect_counts[us][sq] += 1;
        });

        // 打った駒の影の利きを利き数に加える。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_support =
                DirectionSet::from_piece_supported(pc) & self.ranged_effects[dst].get(us);
            dirs_support.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] += 1;
                }
            });
        }

        // 遠隔利きを更新。
        {
            // 打った駒により発生する遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc);

            // 打った駒により遮断される遠隔利き。
            let dsp_others = self.ranged_effects[dst];

            self.update_effect_ranged(dst, dsp_us, dsp_others, true);
        }
    }

    /// 指した側 (`us` とする)が `sq` に駒を置くか、または `sq` から駒を除去したときの
    /// `sq` からの 8 方向の遠隔利きの差分更新処理。両陣営同時に処理される。
    /// 呼び出し時点ではまだ指した側の手番になっている。
    ///
    /// `dsp_us` は指した側が置いた/除去した駒により変化する遠隔利き(`us` でない側は空集合になっている)。
    /// `dsp_others` はこの操作により変化する既存の遠隔利き。これは両陣営混在している。
    ///
    /// `sq` に駒を置く場合、`put` に `true` を渡す。
    /// `sq` から駒を除去する場合、`put` に `false` を渡す。
    fn update_effect_ranged(
        &mut self,
        sq: Square,
        dsp_us: DirectionSetPair,
        dsp_others: DirectionSetPair,
        put: bool,
    ) {
        let us = self.side_to_move; // 指した側
        let them = us.inv();
        let sq_ww = SquareWithWall::from(sq);

        // 駒を置いたか除去したかで利き数の増減値の符号が変わる(後述)。
        let e_sign: i8 = if put { 1 } else { -1 };

        // us 側については、置いた/除去した駒の遠隔利きと既存の遠隔利きは相殺する。
        // them 側は同時に処理するため OR したい。
        // これらは全体を XOR することで表せる (dsp_us の them 側は空集合であることに注意)。
        let mut dsp = dsp_us ^ dsp_others;

        // dsp に含まれる各方向(陣営問わず)について順に更新していく。
        while !dsp.is_empty() {
            // 更新対象のマスに対し dsp_dir を XOR すればよい。
            let (dir, dsp_dir) = dsp.pop();

            // us 側の利き数の増減値を求める。
            // ここで dsp_dir[us][dir] のような記法を導入する。
            // これは、「dsp_dir の us 側に dir が含まれるかどうか」という bool 値を表す。
            //
            // dsp の定義より、dsp_dir[us][dir] == 0 ならば明らかに利きは増減しない。
            // dsp_dir[us][dir] == 1 ならば以下のようになる:
            //
            // * 駒を置いて dsp_us[us][dir] == 1 ならば +1 (発生)
            // * 駒を置いて dsp_us[us][dir] == 0 ならば -1 (遮断)
            // * 駒を除去して dsp_us[us][dir] == 1 ならば -1 (除去)
            // * 駒を除去して dsp_us[us][dir] == 0 ならば +1 (開放)
            let e_us = if dsp_dir.get(us).is_empty() {
                0
            } else if (dsp_us & dsp_dir).is_empty() {
                -e_sign
            } else {
                e_sign
            };

            // them 側の利き数の増減値を求める。
            // これは put と dsp_dir[them][dir] だけ見ればよい:
            //
            // * dsp_dir[them][dir] == 0 ならば利きは増減しない。
            // * 駒を置いて dsp_dir[them][dir] == 1 ならば -1 (遮断)
            // * 駒を除去して dsp_dir[them][dir] == 1 ならば +1 (開放)
            let e_them = if dsp_dir.get(them).is_empty() {
                0
            } else {
                -e_sign
            };

            // dir 方向へ利きが途切れるまで進みつつ更新。
            let delta = dir.to_sqww_delta();
            let mut dst_ww = sq_ww;
            loop {
                dst_ww += delta;

                // 盤面外に出たら終了。
                if !dst_ww.is_on_board() {
                    break;
                }

                // 利き数と遠隔利きを更新。
                let dst = Square::from(dst_ww);
                self.effect_counts[us][dst].wrapping_add_assign(e_us as u8);
                self.effect_counts[them][dst].wrapping_add_assign(e_them as u8);
                self.ranged_effects[dst] ^= dsp_dir;

                // 駒にぶつかったら影の利きを増減して終了。
                let pc_dst = self.board[dst];
                if pc_dst != NO_PIECE {
                    // dir 方向へさらに 1 歩進んだ先が盤面内なら影の利きを考慮する必要がある。
                    let dst2_ww = dst_ww + delta;
                    if dst2_ww.is_on_board() {
                        let dst2 = Square::from(dst2_ww);
                        let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
                        let support_us =
                            pc_dst.side() == us && !dirs_pc_dst.is_disjoint(dsp_dir.get(us));
                        let support_them =
                            pc_dst.side() == them && !dirs_pc_dst.is_disjoint(dsp_dir.get(them));
                        if support_us {
                            self.effect_counts[us][dst2].wrapping_add_assign(e_us as u8);
                        } else if support_them {
                            self.effect_counts[them][dst2].wrapping_add_assign(e_them as u8);
                        }
                    }
                    break;
                }
            }
        }
    }

    /// 指し手を undo する。
    ///
    /// 不正な指し手は渡されないと仮定している。
    pub fn undo_move(&mut self, umv: UndoableMove) {
        debug_assert!(umv.is_valid());

        // 先に手数と手番を戻す。この方が局面の手番と指し手の主体が一致するのでわかりやすいと思う。
        self.side_to_move = self.side_to_move.inv();
        self.ply -= 1;

        if umv.is_drop() {
            self.undo_move_drop(umv);
        } else {
            self.undo_move_walk(umv);
        }

        /* for debug
        {
            let (effect_counts, ranged_effects) = calc_effect(self);
            let ng_count_hum = self.effect_counts[HUM] != effect_counts[HUM];
            let ng_count_com = self.effect_counts[COM] != effect_counts[COM];
            let ng_range = self.ranged_effects != ranged_effects;
            let ng = ng_count_hum || ng_count_com || ng_range;
            if ng {
                eprintln!("[undo_move() failed]");
                eprintln!("{}", self);
                eprintln!("指し手: {}", umv);
            }
            if ng_count_hum {
                eprintln!("--- HUM counts ---");
                eprintln!("{}", self.effect_counts[HUM]);
                eprintln!("{}", effect_counts[HUM]);
            }
            if ng_count_com {
                eprintln!("--- COM counts ---");
                eprintln!("{}", self.effect_counts[COM]);
                eprintln!("{}", effect_counts[COM]);
            }
            if ng_range {
                eprintln!("--- ranged ---");
                eprintln!("{}", self.ranged_effects);
                eprintln!("{}", ranged_effects);
            }
            assert!(!ng);
        }
        */
    }

    /// 盤上の駒を動かす指し手を undo する。
    ///
    /// 呼び出し側で既に手番は戻してあることに注意。
    fn undo_move_walk(&mut self, umv: UndoableMove) {
        let us = self.side_to_move;
        let them = us.inv();

        let src = umv.src();
        let dst = umv.dst();
        let pc_src = umv.piece_src();
        let pc_dst = umv.piece_dst();
        let pc_captured = umv.piece_captured();

        // 移動元は空白でなければならない。
        debug_assert_eq!(self.board[src], NO_PIECE);
        // 移動元の駒は us 側に属さねばならない。
        debug_assert_eq!(pc_src.side(), us);
        // 移動後の駒が実際に存在し、かつそれは us 側に属さねばならない。
        debug_assert_eq!(self.board[dst], umv.piece_dst());
        debug_assert_eq!(self.board[dst].side(), us);
        // 駒取りの場合、取った駒は them 側に属し、かつ us 側の手駒になければならない。
        debug_assert!(pc_captured == NO_PIECE || pc_captured.side() == them);
        debug_assert!(pc_captured == NO_PIECE || self.hands[us][pc_captured.to_raw_kind()] > 0);

        // 移動先から駒を除去し、移動元に駒を戻す。
        self.remove_piece(dst);
        self.put_piece(src, pc_src);

        // 駒取りか?
        if pc_captured == NO_PIECE {
            // 駒取りでない場合、単に利き情報を復元。
            self.revert_effect_by_noncapture(src, dst, pc_src, pc_dst);
        } else {
            // 駒取りの場合

            // 捕獲した駒を移動先に戻す。
            self.put_piece(dst, pc_captured);

            // 捕獲した駒を us 側の手駒から減らす。
            let pr_captured = pc_captured.to_raw_kind();
            self.hands[us][pr_captured] -= 1;

            // 利き情報を復元。
            self.revert_effect_by_capture(src, dst, pc_src, pc_dst, pc_captured);

            // COM 側の駒数を復元。
            if us == HUM {
                self.com_nonking_count += 1;
            } else {
                self.com_nonking_count -= 1;
            }
        }

        // 玉を動かした場合、玉位置を戻す。
        if pc_src.kind() == KING {
            self.king_sq[us] = src;
        }
    }

    /// 盤上の駒を動かす駒取りの指し手の undo による利き情報復元処理。
    /// 呼び出し時点では既に着手前の局面になっている(手番、盤面とも)。
    fn revert_effect_by_capture(
        &mut self,
        src: Square,
        dst: Square,
        pc_src: Piece,
        pc_dst: Piece,
        pc_captured: Piece,
    ) {
        // * 移動元の駒の近接利きが発生する。
        // * 移動先の駒の近接利きが除去される。
        // * 捕獲された駒の近接利きが発生する。
        //
        // * 移動先の駒の遠隔利きが除去される。
        // * 捕獲された駒の遠隔利きが発生する。
        //
        // * 移動先の駒の影の利きが除去される。
        //
        // * 移動元の駒の遠隔利きが発生する。
        // * 移動元のマスに存在していた遠隔利きが遮断される。
        //
        // * 移動元の駒の影の利きが発生する。
        // * 捕獲された駒の影の利きが発生する。
        //
        // 近接利きを除き、update_effect_by_capture() の逆順に処理される。

        let us = self.side_to_move; // 指した側
        let them = us.inv();

        // 移動元の駒の近接利き発生と、移動先の駒の近接利き除去。
        {
            let mut bb_dec = bbs::effect_melee(pc_src, src); // 利き数を減らしたマスたち
            let mut bb_inc = bbs::effect_melee(pc_dst, dst); // 利き数を増やしたマスたち

            let bb_intersect = bb_dec & bb_inc;
            bb_dec ^= bb_intersect;
            bb_inc ^= bb_intersect;

            bb_dec.for_each_square(|sq| {
                self.effect_counts[us][sq] += 1;
            });
            bb_inc.for_each_square(|sq| {
                self.effect_counts[us][sq] -= 1;
            });
        }

        // 捕獲された駒の近接利き発生。
        bbs::effect_melee(pc_captured, dst).for_each_square(|sq| {
            self.effect_counts[them][sq] += 1;
        });

        // 移動先における遠隔利き復元。ここでは遮断処理は起こらなかった。
        {
            // 移動先の駒により発生した遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_dst);

            // 捕獲された駒が持っていた遠隔利き(復元される)。
            let dsp_others = DirectionSetPair::from_piece_ranged(pc_captured);

            // undo の場合、移動先の駒を除去するので、put 引数に false を渡す。
            //
            // hack:
            // この時点では既に着手前の局面になっているため、
            // 例えば角を移動する指し手で移動元に対して偽の影の利きが生じてしまう。
            // そこで、移動元を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_src_orig = std::mem::replace(&mut self.board[src], H_KNIGHT);
            self.update_effect_ranged(dst, dsp_us, dsp_others, false);
            self.board[src] = pc_src_orig;
        }

        // 移動先の駒の影の利き除去。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
            let dirs_support_dst = dirs_pc_dst & self.ranged_effects[dst].get(us);
            dirs_support_dst.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] -= 1;
                }
            });
        }

        // 移動元における遠隔利き復元。
        // 8 方向への移動の場合、移動と逆方向の向きは先後とも復元しなくてよい。
        {
            let mv_dirs = DirectionSet::from_squares(src, dst);
            let dsp_mask = if mv_dirs.is_empty() {
                // 8 方向への移動でないのは桂のみ。
                DirectionSetPair::all()
            } else {
                let mv_dirs_inv = DirectionSet::from(mv_dirs.get_least().inv());
                !DirectionSetPair::new(mv_dirs_inv, mv_dirs_inv)
            };

            // 移動元の駒が持っていた遠隔利き(復元される)。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_src) & dsp_mask;

            // 移動元の既存の遠隔利き(遮断される)。
            let dsp_others = self.ranged_effects[src] & dsp_mask;

            // undo の場合、移動元の駒を置くので、put 引数に true を渡す。
            //
            // hack: この時点では既に着手前の局面になっているため、
            // 例えば金が取られた場合、そこへの飛車の利きが遮断されると、偽の影の利きを除去してしまう。
            // そこで、捕獲された駒を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_captured_orig = std::mem::replace(&mut self.board[dst], H_KNIGHT);
            self.update_effect_ranged(src, dsp_us, dsp_others, true);
            self.board[dst] = pc_captured_orig;
        }

        // 移動元の駒の影の利き発生。
        {
            let src_ww = SquareWithWall::from(src);
            let dirs_pc_src = DirectionSet::from_piece_supported(pc_src);
            let dirs_support_src = dirs_pc_src & self.ranged_effects[src].get(us);
            dirs_support_src.for_each(|dir| {
                let sq_ww = src_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] += 1;
                }
            });
        }

        // 捕獲された駒の影の利き発生。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_captured = DirectionSet::from_piece_supported(pc_captured);
            let dirs_support_captured = dirs_pc_captured & self.ranged_effects[dst].get(them);
            dirs_support_captured.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[them][Square::from(sq_ww)] += 1;
                }
            });
        }
    }

    /// 盤上の駒を動かす駒取りでない指し手の undo による利き情報復元処理。
    /// 呼び出し時点では既に着手前の局面になっている(手番、盤面とも)。
    fn revert_effect_by_noncapture(
        &mut self,
        src: Square,
        dst: Square,
        pc_src: Piece,
        pc_dst: Piece,
    ) {
        // * 移動元の駒の近接利きが発生する。
        // * 移動先の駒の近接利きが除去される。
        //
        // * 移動先の駒の遠隔利きが除去される。
        // * 移動先のマスに存在していた遠隔利きが開放される。
        //
        // * 移動先の駒の影の利きが除去される。
        //
        // * 移動元の駒の遠隔利きが発生する。
        // * 移動元のマスに存在していた遠隔利きが遮断される。
        //
        // * 移動元の駒の影の利きが発生する。

        let us = self.side_to_move; // 指した側

        // 移動元の駒の近接利き発生と、移動先の駒の近接利き除去。
        {
            let mut bb_dec = bbs::effect_melee(pc_src, src); // 利き数を減らしたマスたち
            let mut bb_inc = bbs::effect_melee(pc_dst, dst); // 利き数を増やしたマスたち

            let bb_intersect = bb_dec & bb_inc;
            bb_dec ^= bb_intersect;
            bb_inc ^= bb_intersect;

            bb_dec.for_each_square(|sq| {
                self.effect_counts[us][sq] += 1;
            });
            bb_inc.for_each_square(|sq| {
                self.effect_counts[us][sq] -= 1;
            });
        }

        // 移動先における遠隔利き復元。
        {
            // 移動先の駒により発生した遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_dst);

            // 移動先の駒により遮断された遠隔利き。
            let dsp_others = self.ranged_effects[dst];

            // undo の場合、移動先の駒を除去するので、put 引数に false を渡す。
            //
            // hack:
            // この時点では既に着手前の局面になっているため、
            // 例えば角を移動する指し手で移動元に対して偽の影の利きが生じてしまう。
            // そこで、移動元を一時的に HUM 側の桂に置き換えて問題を回避する。
            let pc_src_orig = std::mem::replace(&mut self.board[src], H_KNIGHT);
            self.update_effect_ranged(dst, dsp_us, dsp_others, false);
            self.board[src] = pc_src_orig;
        }

        // 移動先の駒の影の利き除去。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
            let dirs_support_dst = dirs_pc_dst & self.ranged_effects[dst].get(us);
            dirs_support_dst.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] -= 1;
                }
            });
        }

        // 移動元における遠隔利き復元。
        // 8 方向への移動の場合、移動と逆方向の向きは先後とも復元しなくてよい。
        {
            let mv_dirs = DirectionSet::from_squares(src, dst);
            let dsp_mask = if mv_dirs.is_empty() {
                // 8 方向への移動でないのは桂のみ。
                DirectionSetPair::all()
            } else {
                let mv_dirs_inv = DirectionSet::from(mv_dirs.get_least().inv());
                !DirectionSetPair::new(mv_dirs_inv, mv_dirs_inv)
            };

            // 移動元の駒が持っていた遠隔利き(復元される)。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc_src) & dsp_mask;

            // 移動元の既存の遠隔利き(遮断される)。
            let dsp_others = self.ranged_effects[src] & dsp_mask;

            // undo の場合、移動元の駒を置くので、put 引数に true を渡す。
            self.update_effect_ranged(src, dsp_us, dsp_others, true);
        }

        // 移動元の駒の影の利き発生。
        {
            let src_ww = SquareWithWall::from(src);
            let dirs_pc_src = DirectionSet::from_piece_supported(pc_src);
            let dirs_support_src = dirs_pc_src & self.ranged_effects[src].get(us);
            dirs_support_src.for_each(|dir| {
                let sq_ww = src_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] += 1;
                }
            });
        }
    }

    /// 駒打ちの指し手を undo する。
    ///
    /// 呼び出し側で既に手番は戻してあることに注意。
    fn undo_move_drop(&mut self, umv: UndoableMove) {
        let us = self.side_to_move;
        let pk = umv.dropped_piece_kind();
        let dst = umv.dst();

        // 移動先に該当する駒種が打たれていなければならない。
        debug_assert_eq!(self.board[dst], Piece::new(us, pk));

        // 打った駒を盤上から除去。
        self.remove_piece(dst);

        // 打った駒種を手駒に戻す。
        self.hands[us][pk] += 1;

        // drop では玉位置の更新は起こらない。

        // 利き情報を復元。
        self.revert_effect_by_drop(pk, dst);
    }

    /// 駒打ちの指し手の undo による利き情報復元処理。
    /// 呼び出し時点では既に着手前の局面になっている(手番、盤面とも)。
    fn revert_effect_by_drop(&mut self, pk: PieceKind, dst: Square) {
        let us = self.side_to_move; // 指した側
        let pc = Piece::new(us, pk);

        // 打った駒の近接利きを利き数から引く。
        bbs::effect_melee(pc, dst).for_each_square(|sq| {
            self.effect_counts[us][sq] -= 1;
        });

        // 遠隔利きを復元。
        {
            // 打った駒により発生した遠隔利き。
            let dsp_us = DirectionSetPair::from_piece_ranged(pc);

            // 打った駒により遮断された遠隔利き。
            let dsp_others = self.ranged_effects[dst];

            // undo の場合、打った駒を除去するので、put 引数に false を渡す。
            self.update_effect_ranged(dst, dsp_us, dsp_others, false);
        }

        // 打った駒の影の利きを利き数から引く。
        {
            let dst_ww = SquareWithWall::from(dst);
            let dirs_support =
                DirectionSet::from_piece_supported(pc) & self.ranged_effects[dst].get(us);
            dirs_support.for_each(|dir| {
                let sq_ww = dst_ww + dir.to_sqww_delta();
                if sq_ww.is_on_board() {
                    self.effect_counts[us][Square::from(sq_ww)] -= 1;
                }
            });
        }
    }

    /// 指定した陣営が王手をかけられているかどうかを返す。
    pub fn is_checked(&self, us: Side) -> bool {
        let them = us.inv();

        self.effect_counts[them][self.king_sq[us]] > 0
    }

    /// 手番の側がチェックメイト(**打ち歩含む**)されているかどうかを返す。
    ///
    /// 関数から戻ったとき、`self` は呼び出し前の局面に戻っている。
    pub fn is_checkmated(&mut self) -> bool {
        position_is_checkmated(self)
    }

    /// `sq` に `pc` を置き、bitboard たちも合わせて更新する。
    /// `sq` は空白でなければならない。
    fn put_piece(&mut self, sq: Square, pc: Piece) {
        debug_assert_eq!(self.board[sq], NO_PIECE);

        self.board[sq] = pc;
        self.xor_piece(sq, pc);
    }

    /// `sq` にある駒を除去し、bitboard たちも合わせて更新する。
    /// `sq` には実際の駒がなければならない。
    fn remove_piece(&mut self, sq: Square) {
        let pc = self.board[sq];
        debug_assert!(pc.is_piece());

        self.board[sq] = NO_PIECE;
        self.xor_piece(sq, pc);
    }

    /// `put_piece()`, `remove_piece()` 内での bitboard 更新処理。
    fn xor_piece(&mut self, sq: Square, pc: Piece) {
        self.bb_occ ^= Bitboard::from(sq);
        self.bb_occ_side[pc.side()] ^= Bitboard::from(sq);
        self.bb_pk[pc.kind()] ^= Bitboard::from(sq);
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "COM 手駒: {}", self.hands[COM])?;
        write!(f, "{}", self.board)?;
        writeln!(f, "HUM 手駒: {}", self.hands[HUM])?;
        writeln!(f, "手番: {}", self.side_to_move)?;

        Ok(())
    }
}

/// 局面から `EffectCountBoards`, `RangedEffectBoard` を愚直に計算する。
/// `Position` の初期化時のみ使う。
fn calc_effect(pos: &Position) -> (EffectCountBoards, RangedEffectBoard) {
    let mut effect_counts =
        EffectCountBoards::from([EffectCountBoard::empty(), EffectCountBoard::empty()]);
    let mut ranged_effects = RangedEffectBoard::empty();

    // 盤面上の駒全てについて利きを処理する。
    pos.bb_occupied().for_each_square(|src| {
        let pc = pos.board()[src];
        let side = pc.side();
        let eff = bbs::effect(pc, src, pos.bb_occupied());

        // 全ての近接利きを利き数に加算。

        // 香、角、飛車は近接利きを持たない。
        // 馬、龍は専用の近接利きを持つ。
        // 近接駒は eff がそのまま近接利きとなる。
        let eff_melee = match pc {
            H_LANCE | C_LANCE | H_BISHOP | C_BISHOP | H_ROOK | C_ROOK => Bitboard::zero(),
            H_HORSE | C_HORSE => bbs::axis_cross_effect(src),
            H_DRAGON | C_DRAGON => bbs::diagonal_cross_effect(src),
            _ => eff,
        };
        eff_melee.for_each_square(|dst| {
            effect_counts[side][dst] += 1;
        });

        // 全ての遠隔利きを処理。
        // 遠隔利きおよびそれによる影の利きを利き数に加算。
        // 遠隔利きそのものを ranged_effects に追加。

        // 香、角、飛車は eff がそのまま遠隔利きとなる。
        // 馬、龍は eff から近接利きを除いたものが遠隔利き。
        // 近接駒は遠隔利きを持たないので、ここで処理を打ち切る。
        let eff_ranged = match pc {
            H_LANCE | C_LANCE | H_BISHOP | C_BISHOP | H_ROOK | C_ROOK => eff,
            H_HORSE | C_HORSE => bbs::axis_cross_effect(src).andnot(eff),
            H_DRAGON | C_DRAGON => bbs::diagonal_cross_effect(src).andnot(eff),
            _ => return,
        };
        eff_ranged.for_each_square(|dst| {
            effect_counts[side][dst] += 1;
            let dirs = DirectionSet::from_squares(src, dst);
            let dsp = DirectionSetPair::from_part(side, dirs);
            // 同じ陣営の同じ方向の遠隔利きが重なることはないから OR でなく XOR でよい。
            // バグらせたとき検出できる可能性がある点で OR より優る。
            ranged_effects[dst] ^= dsp;
        });

        // 影の利きを利き数に加算。
        // 桂、玉の上では影の利きが発生しないので、
        // 遠隔利きを桂、玉以外の自駒の occupied bitboard でマスクし、1 マスずつ処理する。
        let bb_support = pos.bb_piece_kind(KNIGHT).andnot(
            pos.bb_piece_kind(KING)
                .andnot(eff_ranged & pos.bb_occupied_side(side)),
        );
        bb_support.for_each_square(|dst| {
            // dst にある自駒が遠隔利きと同じ方向の利きを持つなら、遠隔利きを 1 歩延長。
            let pc_dst = pos.board()[dst];
            let dirs_eff = DirectionSet::from_squares(src, dst);
            let dirs_pc_dst = DirectionSet::from_piece_supported(pc_dst);
            if !dirs_eff.is_disjoint(dirs_pc_dst) {
                let dir_eff = dirs_eff.get_least();
                let sq_ww = SquareWithWall::from(dst) + dir_eff.to_sqww_delta();
                if sq_ww.is_on_board() {
                    effect_counts[side][Square::from(sq_ww)] += 1;
                }
            }
        });
    });

    (effect_counts, ranged_effects)
}
