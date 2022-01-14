//! 思考エンジン。
//!
//! 原作でオーバーフローが起こりうる箇所には wrapping 演算を使っている。

use std::cmp::Ordering;

use anyhow::bail;

use crate::bbs;
use crate::book::{BookState, Formation};
use crate::movegen::{generate_moves_com, position_is_checkmated_naitou};
use crate::myarray::*;
use crate::mylog::*;
use crate::mynum::{WrappingAddAssign, WrappingSubAssign};
use crate::naitou::*;
use crate::position::Position;
use crate::shogi::*;
use crate::util;

/// HUM 側の指し手に対する思考エンジンの応答。undo 用情報も含む。
#[derive(Debug)]
pub enum EngineResponse {
    /// 通常の指し手。
    Move(EngineResponseMove),

    /// HUM の勝ち(エンジンが投了)。COM の指し手を含まない。
    HumWin(EngineResponseHumWin),

    /// COM の勝ち(HUM が自殺手を指した)。COM の指し手を含まない。
    HumSuicide(EngineResponseHumSuicide),

    /// COM の勝ち(COM の指し手で HUM 玉が詰んだ)。
    ComWin(EngineResponseComWin),
}

impl EngineResponse {
    fn new_move(umv_com: UndoableMove, undo_info: EngineUndoInfo) -> Self {
        Self::Move(EngineResponseMove { umv_com, undo_info })
    }

    fn new_hum_win(undo_info: EngineUndoInfo) -> Self {
        Self::HumWin(EngineResponseHumWin { undo_info })
    }

    fn new_hum_suicide(undo_info: EngineUndoInfo) -> Self {
        Self::HumSuicide(EngineResponseHumSuicide { undo_info })
    }

    fn new_com_win(umv_com: UndoableMove, undo_info: EngineUndoInfo) -> Self {
        Self::ComWin(EngineResponseComWin { umv_com, undo_info })
    }

    /// 応答に COM の指し手が含まれればそれを返す。
    pub fn move_com(&self) -> Option<UndoableMove> {
        match self {
            Self::Move(res) => Some(res.umv_com),
            Self::HumWin(_) => None,
            Self::HumSuicide(_) => None,
            Self::ComWin(res) => Some(res.umv_com),
        }
    }

    /// 保持する `EngineUndoInfo` への参照を返す。
    fn undo_info(&self) -> &EngineUndoInfo {
        match self {
            Self::Move(res) => &res.undo_info,
            Self::HumWin(res) => &res.undo_info,
            Self::HumSuicide(res) => &res.undo_info,
            Self::ComWin(res) => &res.undo_info,
        }
    }
}

#[derive(Debug)]
pub struct EngineResponseMove {
    umv_com: UndoableMove, // COM 側の指し手。
    undo_info: EngineUndoInfo,
}

impl EngineResponseMove {
    /// COM の指し手を返す。
    pub fn move_com(&self) -> UndoableMove {
        self.umv_com
    }
}

#[derive(Debug)]
pub struct EngineResponseHumWin {
    undo_info: EngineUndoInfo,
}

#[derive(Debug)]
pub struct EngineResponseHumSuicide {
    undo_info: EngineUndoInfo,
}

#[derive(Debug)]
pub struct EngineResponseComWin {
    umv_com: UndoableMove, // COM 側の指し手。
    undo_info: EngineUndoInfo,
}

impl EngineResponseComWin {
    /// COM の指し手を返す。
    pub fn move_com(&self) -> UndoableMove {
        self.umv_com
    }
}

/// 直前の局面の情報 (undo 用)。
#[derive(Debug)]
struct EngineUndoInfo {
    umv_hum: UndoableMove, // HUM 側の指し手。
    progress_ply: u8,
    progress_level: u8,
    progress_level_sub: u8,
    book_state: BookState,
    naitou_best_src_value: u8,
}

/// HUM 側の指し手に対する思考エンジンの応答。undo 用情報を含まない。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum EngineResponseRaw {
    /// 指し手(COM の指し手で HUM 玉が詰んだケースを含む)。
    Move(EngineResponseRawMove),

    /// HUM の勝ち(エンジンが投了)。
    HumWin,

    /// COM の勝ち(HUM が自殺手を指した)。
    HumSuicide,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct EngineResponseRawMove {
    best_mv: Move,
    quiet: bool, // ルート局面で駒得マスも駒損マスもなく、かつ最善手が駒取りでない
    force_skip_book: bool, // 定跡処理を強制的にスキップ
    hum_is_checkmated: bool, // 最善手で HUM 玉が詰む
}

/// ルート局面(思考開始局面)の評価。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RootEvaluation {
    /// 最大駒得マスの HUM 駒の価値。
    pub adv_price: u8,

    /// 最大駒損マスの COM 駒の価値。
    pub disadv_price: u8,

    /// HUM 側の `8*(持飛+持角+成駒) + 4*(持金+持銀) + 2*(持桂+持香) + 1*(持歩) + (手数補正)`
    pub power_hum: u8,

    /// COM 側の `8*(持飛+持角+成駒) + 4*(持金+持銀) + 2*(持桂+持香) + 1*(持歩) + (手数補正)`
    pub power_com: u8,

    /// COM 側の `(持飛) + (持角) + (成駒)`
    pub rbp_com: u8,

    /// 両陣営の玉位置。末端局面評価時に使われるので、便宜上ここに含める。
    pub king_sq: MyArray1<Square, Side, 2>,
}

/// 末端局面(候補手を指した局面)の評価。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LeafEvaluation {
    /// 候補手で捕獲する駒の価値(駒取りでなければ 0)を補正した値。
    pub capture_price: u8,

    /// 最大駒得マスの HUM 駒の価値を補正した値。
    pub adv_price: u8,

    /// 最大駒得マス。
    pub adv_sq: Option<Square>,

    /// 最大駒損マスの COM 駒の価値を補正した値。
    pub disadv_price: u8,

    /// 最大駒損マス。
    pub disadv_sq: Option<Square>,

    /// 全駒得マスの HUM 駒の価値の総和を補正した値。
    pub score_posi: u8,

    /// 全駒損マスの COM 駒の価値の総和を補正した値。
    pub score_nega: u8,

    /// *ルート局面での* HUM 玉位置から距離 2 以内のマスへの COM 利き数の総和。
    pub hum_king_threat_around25: u8,

    /// *ルート局面での* COM 玉位置から距離 2 以内のマスへの COM 利き数の総和。
    pub com_king_safety_around25: u8,

    /// *ルート局面での* COM 玉位置から距離 2 以内のマスへの HUM 利き数の総和。
    pub com_king_threat_around25: u8,

    /// *ルート局面での* COM 玉位置からちょうど距離 1 のマスへの HUM 利き数の総和。
    pub com_king_threat_around8: u8,

    /// *ルート局面での* COM 玉位置からちょうど距離 1 で、`(HUM 利き数) >= (COM 利き数)` なるマスの個数。
    pub com_king_choke_count_around8: u8,

    /// 候補手の移動元から *ルート局面での* COM 玉位置への距離。
    /// ただし候補手が駒打ちの場合、移動先からの距離。
    pub src_to_com_king: u8,

    /// 候補手の移動先から *ルート局面での* HUM 玉位置への距離。
    pub dst_to_hum_king: u8,

    /// HUM 側の垂れ歩または垂れ香が存在するかどうか。
    pub hum_hanging: bool,

    /// COM 側の成駒の個数。
    pub com_promo_count: u8,

    /// COM 側の離れ駒の個数。ただし歩、香、桂、玉は対象外。
    pub com_loose_count: u8,

    /// HUM 玉が詰みかどうか。
    pub hum_is_checkmated: bool,

    /// COM が自殺手を指したかどうか(利きを見て判定)。
    /// この項目は原作には存在しないが、取り返しフラグの影響で COM 玉が取れてしまうことがあり、
    /// その対策として入れてある(本プログラムは玉を取る手には対応していないので)。
    pub is_suicide: bool,
}

impl LeafEvaluation {
    /// 局面評価開始時の初期値を返す。
    fn new() -> Self {
        Self {
            capture_price: 0,
            adv_price: 0,
            adv_sq: None,
            disadv_price: 0,
            disadv_sq: None,
            score_posi: 0,
            score_nega: 0,
            hum_king_threat_around25: 0,
            com_king_safety_around25: 0,
            com_king_threat_around25: 0,
            com_king_threat_around8: 0,
            com_king_choke_count_around8: 0,
            src_to_com_king: 0,
            dst_to_hum_king: 0,
            hum_hanging: false,
            com_promo_count: 0,
            com_loose_count: 0,
            hum_is_checkmated: false,
            is_suicide: false,
        }
    }

    /// 最悪の評価を返す。どの候補手もこれよりは良い評価になる、はず。
    fn worst() -> Self {
        // (未使用) と書かれた項目は最善手の評価としては使われない。

        Self {
            capture_price: 0,
            adv_price: 0,
            adv_sq: None, // (未使用)
            disadv_price: 99,
            disadv_sq: None, // (未使用)
            score_posi: 0,
            score_nega: 99,
            hum_king_threat_around25: 0,
            com_king_safety_around25: 0,
            com_king_threat_around25: 99,
            com_king_threat_around8: 99,      // (未使用)
            com_king_choke_count_around8: 99, // (未使用)
            src_to_com_king: 0,
            dst_to_hum_king: 99,
            hum_hanging: true, // (未使用)
            com_promo_count: 0,
            com_loose_count: 99,
            hum_is_checkmated: false,
            is_suicide: false,
        }
    }
}

/// 原作を再現した思考エンジン。
///
/// 基本的に HUM の手番の局面を保持する。
/// ただし、COM が投了した場合と、HUM が自殺手を指した場合は例外。
#[derive(Clone, Debug)]
pub struct Engine {
    pos: Position,
    progress_ply: u8,       // 進行度管理用の手数 (0..=100)。開始局面では 0。
    progress_level: u8,     // 進行度 (0..=3)。
    progress_level_sub: u8, // サブ進行度 (0..=5)。進行度 0 のときのみ使われる。
    book_state: BookState,

    // 駒打ちの候補手と最善手を比較する際に必要となる値。
    // 原作ではこの値が局面ごとに初期化されないため、値を保持しておく必要がある。
    naitou_best_src_value: u8,
}

impl Engine {
    /// 指定した手合割で初期化された思考エンジンを返す。
    /// COM が先に指す手合割の場合、COM の着手も行い、その指し手も返す。
    pub fn new(handicap: Handicap) -> (Self, Option<UndoableMove>) {
        let (side_to_move, board, hands) = handicap.startpos();
        let pos = Position::new(side_to_move, board, hands);

        let formation = Formation::from_handicap(handicap);
        let book_state = BookState::new(formation);

        let mut engine = Self {
            pos,
            progress_ply: 0,
            progress_level: 0,
            progress_level_sub: 0,
            book_state,
            naitou_best_src_value: 0,
        };

        // COM が先に指す場合、その着手を行い、指し手を取得する。
        let umv_com = if engine.pos.side_to_move() == HUM {
            None
        } else {
            let resp_raw = engine.think(None);
            // 初手は通常の指し手のはず。
            if let EngineResponseRaw::Move(resp_raw_move) = resp_raw {
                // TODO: ログ出力コードが分散して汚いのでどうにかしたいが...
                log_engine_response_move(resp_raw_move.best_mv);
                log_think_end();
                Some(engine.do_move_com(resp_raw_move.best_mv))
            } else {
                panic!(
                    "the first move should be a normal move, but got: {:?}",
                    resp_raw
                );
            }
        };

        (engine, umv_com)
    }

    /// 現在の局面への参照を返す。
    #[inline]
    pub fn position(&self) -> &Position {
        &self.pos
    }

    /// 進行度管理用の手数を返す。
    pub fn progress_ply(&self) -> u8 {
        self.progress_ply
    }

    /// 進行度を返す。
    pub fn progress_level(&self) -> u8 {
        self.progress_level
    }

    /// サブ進行度を返す。
    pub fn progress_level_sub(&self) -> u8 {
        self.progress_level_sub
    }

    /// 保持する `BookState` への参照を返す。
    pub fn book_state(&self) -> &BookState {
        &self.book_state
    }

    /// HUM 側の指し手とそれに対する COM の応手(あれば)で局面を進め、思考エンジンの応答を返す。
    ///
    /// `mv_hum` は少なくとも疑似合法手でなければならない。これが自殺手の場合、エラーを返す。
    /// (原作通り、HUM 側の打ち歩詰めは許される)
    ///
    /// `self` が保持する局面は HUM の手番でなければならない。
    pub fn do_step(&mut self, mv_hum: Move) -> anyhow::Result<EngineResponse> {
        let undo_info = self.do_move_hum(mv_hum)?;

        let resp_raw = self.think(Some(mv_hum));

        let resp = match resp_raw {
            EngineResponseRaw::Move(resp_raw_move) => {
                let mv_com = resp_raw_move.best_mv;
                let umv_com = self.do_move_com(mv_com);
                // 最善手を指した局面で HUM 玉が詰みなら COM 勝ち。
                if resp_raw_move.hum_is_checkmated {
                    log_engine_response_com_win(mv_com);
                    EngineResponse::new_com_win(umv_com, undo_info)
                } else {
                    log_engine_response_move(mv_com);
                    EngineResponse::new_move(umv_com, undo_info)
                }
            }
            EngineResponseRaw::HumWin => {
                log_engine_response_hum_win();
                EngineResponse::new_hum_win(undo_info)
            }
            EngineResponseRaw::HumSuicide => {
                log_engine_response_hum_suicide();
                EngineResponse::new_hum_suicide(undo_info)
            }
        };

        log_think_end();

        Ok(resp)
    }

    /// COM 側の局面で思考を行い、`EngineResponseRaw` を返す。局面は進めない。
    #[inline]
    fn think(&mut self, mv_hum: Option<Move>) -> EngineResponseRaw {
        log_think_start(self.pos.ply());
        log_position(self.pos.side_to_move(), self.pos.board(), self.pos.hands());
        log_effect_count_board(HUM, self.pos.effect_count_board(HUM));
        log_effect_count_board(COM, self.pos.effect_count_board(COM));
        log_progress(
            self.progress_ply,
            self.progress_level,
            self.progress_level_sub,
        );
        log_formation(self.book_state.formation());

        // ルート局面を評価。
        let root_eval = self.evaluate_root();
        log_root_evaluation(&root_eval);

        // 探索による思考を行う。
        let resp_raw = self.think_search(&root_eval);

        // 以下の条件を全て満たすとき、think_search() の結果によらず定跡処理を行う:
        //
        // * progress_ply <= 6
        // * HUM の指し手の移動先が２二, ４五, ５六のいずれか
        // * progress_level == 0
        //
        // XXX: 原作ではこれにより王手放置ができてしまう手順がある。
        // 本プログラムでは HUM 側の自殺手は生成しないので影響はない。
        if let Some(mv_hum) = mv_hum {
            let dst = mv_hum.dst();
            if self.progress_ply <= 6
                && (dst == SQ_22 || dst == SQ_45 || dst == SQ_56)
                && self.progress_level == 0
            {
                if let Some(book_mv) = self.think_book(Some(mv_hum)) {
                    return EngineResponseRaw::Move(EngineResponseRawMove {
                        best_mv: book_mv,
                        quiet: false,           // 使われない
                        force_skip_book: false, // 使われない
                        hum_is_checkmated: false,
                    });
                }
                // 定跡手が尽きたら進行度 1 とする(もう定跡が使われることはない)。
                self.progress_level = 1;
            }
        }

        // 探索により指し手が返された場合、定跡も検討する。
        if let EngineResponseRaw::Move(resp_raw_move) = resp_raw {
            // 進行度 0 のとき、quiet でない指し手が返されるたびにサブ進行度を進める。
            // サブ進行度が 5 になったら進行度 1 とする。
            if self.progress_level == 0 && !resp_raw_move.quiet {
                self.progress_level_sub += 1;
                if self.progress_level_sub >= 5 {
                    self.progress_level = 1;
                }
            }

            // 以下の条件を全て満たすとき定跡処理を行う:
            //
            // * 進行度が 0
            // * 指し手が quiet
            // * 定跡処理強制スキップフラグが立っていない
            if self.progress_level == 0 && resp_raw_move.quiet && !resp_raw_move.force_skip_book {
                if let Some(book_mv) = self.think_book(mv_hum) {
                    return EngineResponseRaw::Move(EngineResponseRawMove {
                        best_mv: book_mv,
                        quiet: false,           // 使われない
                        force_skip_book: false, // 使われない
                        hum_is_checkmated: false,
                    });
                }
                // 定跡手が尽きたら進行度 1 とする(もう定跡が使われることはない)。
                self.progress_level = 1;
            }
        }

        resp_raw
    }

    /// 探索による思考(定跡を使わない)。
    #[inline]
    fn think_search(&mut self, root_eval: &RootEvaluation) -> EngineResponseRaw {
        // ルート局面での最大駒得スコアが閾値以上なら HUM 玉が取れる、即ち HUM が自殺手を指したと判定。
        // 原作ではこの後に他の判定もあるが、それは冗長なので省いてよい。
        if root_eval.adv_price >= 30 {
            return EngineResponseRaw::HumSuicide;
        }

        // 最善手とその評価の初期値。どの候補手もこの評価よりは良い、はず。
        let mut best_mv: Option<Move> = None;
        let mut best_eval = LeafEvaluation::worst();

        // 全候補手を生成し、順に試す。
        let mut done = false;
        for mv in generate_moves_com(&self.pos) {
            // 候補手を適用した末端局面を評価する。
            let umv = self.pos.do_move(mv);

            log_cand_start(mv);
            log_board(self.pos.board());
            log_effect_count_board(HUM, self.pos.effect_count_board(HUM));
            log_effect_count_board(COM, self.pos.effect_count_board(COM));

            let leaf_eval = self.evaluate_leaf(root_eval, umv);

            // 候補手が却下されていなければ、評価修正および最善手との比較を行う。
            if let Some(mut leaf_eval) = leaf_eval {
                self.revise_leaf_evaluation(root_eval, umv, &mut leaf_eval);
                log_leaf_evaluation_revised(&leaf_eval);

                let hum_is_checkmated = leaf_eval.hum_is_checkmated;
                if hum_is_checkmated {
                    log_hum_is_checkmated();
                } else {
                    log_cmp_start();
                }

                if hum_is_checkmated
                    || self.can_improve_best(root_eval, &best_eval, &leaf_eval, umv)
                {
                    best_mv = Some(mv);
                    best_eval = leaf_eval;
                    // naitou_best_src_value を更新する。
                    // 候補手と最善手の比較が正しくできれば良いので、
                    // 盤上の駒を動かす手の場合は 0 としておけばよい(値自体は原作とは異なる)。
                    self.naitou_best_src_value = if umv.is_drop() {
                        naitou_com_drop_src_value(umv.dropped_piece_kind())
                    } else {
                        0
                    };
                }

                // HUM 玉の詰みが見つかったら打ち切る。
                if hum_is_checkmated {
                    done = true;
                }
            }

            log_best(best_mv, &best_eval);
            log_cand_end();

            self.pos.undo_move(umv);

            if done {
                break;
            }
        }

        // 最善手を指した局面で COM 玉が取られるなら HUM 勝ち。
        //
        // XXX: 原作では、取り返しフラグ補正が大量にかかった場合 disadv_price が閾値を下回り、
        // COM 玉が詰んでいるのに投了せず、COM 玉が取れてしまうことがある。
        // (実際には取っても何も起こらず、次の手で COM 玉が復活する)
        // 本プログラムは玉を取る手には対応していないので、
        // 最善手が自殺手の場合も HUM 勝ちにするという条件を追加している。
        if best_eval.disadv_price >= 31 || best_eval.is_suicide {
            return EngineResponseRaw::HumWin;
        }

        // HUM が自殺手を指しておらず、HUM 勝ちでもなければ指し手を返す。
        // 少なくとも 1 つの最善手があるはず。
        let best_mv = best_mv.expect("at least 1 move should be accepted");

        // ルート局面で駒損マスも駒得マスもなく、かつ最善手が駒取りでなければ "quiet" である。
        let quiet =
            root_eval.adv_price == 0 && root_eval.disadv_price == 0 && best_eval.capture_price == 0;

        // 有望な駒得マスが複数あると考えられる場合、定跡処理を強制的にスキップする。
        let force_skip_book =
            best_eval.score_posi != best_eval.adv_price && best_eval.score_posi >= 8;

        EngineResponseRaw::Move(EngineResponseRawMove {
            best_mv,
            quiet,
            force_skip_book,
            hum_is_checkmated: best_eval.hum_is_checkmated,
        })
    }

    /// ルート局面を評価する。
    #[inline]
    fn evaluate_root(&self) -> RootEvaluation {
        // 最大駒得マスの HUM 駒の価値を求める。駒得マスがない場合は 0 とする。
        let adv_price = self
            .iter_advantage_squares()
            .map(|(_, pk)| naitou_piece_price_b(pk))
            .max()
            .unwrap_or(0);

        // 最大駒損マスの COM 駒の価値を求める。駒損マスがない場合は 0 とする。
        // 取り返しフラグにより補正がかかる。
        //
        // XXX: 場合によっては取り返しフラグの影響で disadv_price が大きく減少することもありうる。
        // (先に max をとっているので、オーバーフローはしないはず)
        let mut disadv_price = 0;
        for (_, pk, exchange) in self.iter_disadvantage_squares() {
            util::chmax(&mut disadv_price, naitou_piece_price_d(pk));
            if exchange {
                disadv_price -= 1;
            }
        }

        // 双方の成駒を数える。
        let bb_promo = self.pos.bb_piece_kind(PRO_PAWN)
            | self.pos.bb_piece_kind(PRO_LANCE)
            | self.pos.bb_piece_kind(PRO_KNIGHT)
            | self.pos.bb_piece_kind(PRO_SILVER)
            | self.pos.bb_piece_kind(HORSE)
            | self.pos.bb_piece_kind(DRAGON);
        let hum_promo_count = (bb_promo & self.pos.bb_occupied_side(HUM)).count_ones();
        let com_promo_count = (bb_promo & self.pos.bb_occupied_side(COM)).count_ones();

        // power, rbp を求める。
        // power は u8 の範囲ではオーバーフローしうる。(u32 からのキャストで同様の結果になる)

        let hand_hum = self.pos.hand(HUM);
        let hand_com = self.pos.hand(COM);

        // 手数補正。77 手目以降では 2 倍になる。
        let mut ply_factor = u32::from(self.progress_ply) / 11;
        if ply_factor >= 7 {
            ply_factor *= 2;
        }

        let power_hum = (8 * (hum_promo_count + hand_hum[ROOK] + hand_hum[BISHOP])
            + 4 * (hand_hum[GOLD] + hand_hum[SILVER])
            + 2 * (hand_hum[KNIGHT] + hand_hum[LANCE])
            + hand_hum[PAWN]
            + ply_factor) as u8;

        let rbp_com = (com_promo_count + hand_com[ROOK] + hand_com[BISHOP]) as u8;

        let power_com = (8 * u32::from(rbp_com)
            + 4 * (hand_com[GOLD] + hand_com[SILVER])
            + 2 * (hand_com[KNIGHT] + hand_com[LANCE])
            + hand_com[PAWN]
            + ply_factor) as u8;

        let king_sq = MyArray1::<Square, Side, 2>::from([
            self.pos.king_square(HUM),
            self.pos.king_square(COM),
        ]);

        RootEvaluation {
            adv_price,
            disadv_price,
            power_hum,
            power_com,
            rbp_com,
            king_sq,
        }
    }

    /// 末端局面を評価する。候補手が却下された場合、`None` を返す。
    ///
    /// 却下される候補手は以下の通り:
    ///
    /// * 打ち歩詰め
    /// * 駒捨て(移動先が駒損マスで、かつ駒取りでない手)。ただし王手対応や詰ます手は除く
    #[inline]
    fn evaluate_leaf(
        &mut self,
        root_eval: &RootEvaluation,
        umv: UndoableMove,
    ) -> Option<LeafEvaluation> {
        let hum_king_sq = root_eval.king_sq[HUM];
        let com_king_sq = root_eval.king_sq[COM];

        let mut leaf_eval = LeafEvaluation::new();
        let mut sacrifice = false; // 駒捨てかどうか

        leaf_eval.capture_price = {
            let pc = umv.piece_captured();
            if pc == NO_PIECE {
                0
            } else {
                naitou_piece_price_a(pc.kind())
            }
        };

        // 駒得マスによる評価。
        for (sq, pk) in self.iter_advantage_squares() {
            let price = naitou_piece_price_b(pk);

            // 駒得マス上の駒価値の和をとっていく。
            leaf_eval.score_posi.wrapping_add_assign(price);

            // 最大駒得マス更新処理。
            if util::chmax(&mut leaf_eval.adv_price, price) {
                leaf_eval.adv_sq = Some(sq);
            }
        }

        // 駒損マスによる評価。駒捨て判定も行う。
        for (sq, pk, exchange) in self.iter_disadvantage_squares() {
            // 移動先が駒損マスで、かつ駒取りでなければ駒捨て。
            if umv.dst() == sq && leaf_eval.capture_price == 0 {
                sacrifice = true;
            }

            let price = naitou_piece_price_d(pk);

            // 駒損マス上の駒価値の和をとっていく。
            leaf_eval.score_nega.wrapping_add_assign(price);

            // 最大駒損マス更新処理。
            if util::chmax(&mut leaf_eval.disadv_price, price) {
                leaf_eval.disadv_sq = Some(sq);
            }

            // 取り返しフラグによる補正。オーバーフローは起こらないはず。
            if exchange {
                leaf_eval.score_nega -= 1;
                leaf_eval.disadv_price -= 1;
            }
        }

        // 以下の条件を全て満たすとき、HUM 玉の詰み判定を行う(打ち歩詰めは即却下する):
        //
        // * disadv_price < 30 (COM 玉が取られない)
        // * adv_price >= 30 (HUM 玉に王手がかかっている)
        // * 候補手の移動先から HUM 玉への距離が 3 未満
        //
        // XXX: 取り返しフラグ補正が大量にかかって disadv_price が 30 未満になった場合、
        // COM が自玉への王手を無視して詰み判定を行うケースがありうる。かなり稀だろうが...。
        //
        // XXX: 遠隔駒による王手の場合、本来 HUM 玉が詰んでいるのに詰み判定が行われないケースがありうる。
        // ただしこれは HUM が次に自殺手を指して終了するだけなのであまり問題にはならない。
        leaf_eval.hum_is_checkmated = leaf_eval.disadv_price < 30
            && leaf_eval.adv_price >= 30
            && umv.dst().distance(hum_king_sq) < 3
            && position_is_checkmated_naitou(&mut self.pos);
        if leaf_eval.hum_is_checkmated {
            if umv.is_drop() && umv.dropped_piece_kind() == PAWN {
                // 打ち歩詰めは即却下。
                log_cand_reject_by_drop_pawn_mate();
                return None;
            } else {
                // HUM 玉が詰みなら、他の候補手に上書きされないよう評価を修正する。
                //
                // TODO: これ、原作では本当に上書きされないかちょっと自信なし。
                // 原作では詰み判定が出てももう少し指し手生成が続くことがあり、
                // そのときこの手の score_nega が大きすぎると上書きされかねない。
                // あるとしても極めて稀なケースだろうが…。
                //
                // 本プログラムでは詰み判定が出た時点で最善手として採用して打ち切るので、
                // 他の候補手に上書きされることはない。
                leaf_eval.adv_price = 60;
                leaf_eval.capture_price = 60;
                leaf_eval.disadv_price = 0;
            }
        }

        // 王手対応や詰ます手でなければ駒捨ては却下。
        if sacrifice && root_eval.disadv_price < 30 && !leaf_eval.hum_is_checkmated {
            log_cand_reject_by_sacrifice();
            return None;
        }

        // HUM 玉周りの安全度評価。ルート局面での玉位置を用いる(原作通り)。
        bbs::around25(hum_king_sq).for_each_square(|sq| {
            leaf_eval
                .hum_king_threat_around25
                .wrapping_add_assign(self.pos.effect_count_board(COM)[sq]);
        });

        // COM 玉周りの安全度評価。ルート局面での玉位置を用いる(原作通り)。
        bbs::around25(com_king_sq).for_each_square(|sq| {
            leaf_eval
                .com_king_safety_around25
                .wrapping_add_assign(self.pos.effect_count_board(COM)[sq]);
            leaf_eval
                .com_king_threat_around25
                .wrapping_add_assign(self.pos.effect_count_board(HUM)[sq]);
        });
        bbs::king_effect(com_king_sq).for_each_square(|sq| {
            let eff_hum = self.pos.effect_count_board(HUM)[sq];
            let eff_com = self.pos.effect_count_board(COM)[sq];
            leaf_eval
                .com_king_threat_around8
                .wrapping_add_assign(eff_hum);
            if eff_hum >= eff_com {
                leaf_eval.com_king_choke_count_around8 += 1;
            }
        });

        // 指し手と互いの玉との位置関係を評価。
        leaf_eval.src_to_com_king = if umv.is_drop() {
            umv.dst().distance(com_king_sq)
        } else {
            umv.src().distance(com_king_sq)
        };
        leaf_eval.dst_to_hum_king = umv.dst().distance(hum_king_sq);

        // HUM 側の垂れ歩/垂れ香の評価。敵陣 1 段目の歩/香は存在しないと仮定している。
        {
            // HUM 側の歩/香が 4 段目までに存在し、
            // かつその 1 つ上のマスで COM の利きが負けていたら垂れ歩/垂れ香が存在するとみなす。
            let bb_pawn_lance = bbs::forward_rows(HUM, ROW_5)
                & self.pos.bb_occupied_side(HUM)
                & (self.pos.bb_piece_kind(PAWN) | self.pos.bb_piece_kind(LANCE));
            let mut bb = bb_pawn_lance.logical_shift_right_parts::<1>();
            while !bb.is_zero() {
                let sq = bb.pop_least_square();
                let eff_hum = self.pos.effect_count_board(HUM)[sq];
                let eff_com = self.pos.effect_count_board(COM)[sq];
                if eff_hum > eff_com {
                    leaf_eval.hum_hanging = true;
                    break;
                }
            }
        }

        // COM 側の離れ駒をカウント。ただし歩、香、桂、玉は対象外。
        {
            let bb = (self.pos.bb_piece_kind(PAWN)
                | self.pos.bb_piece_kind(LANCE)
                | self.pos.bb_piece_kind(KNIGHT)
                | self.pos.bb_piece_kind(KING))
            .andnot(self.pos.bb_occupied_side(COM));
            bb.for_each_square(|sq| {
                if self.pos.effect_count_board(COM)[sq] == 0 {
                    leaf_eval.com_loose_count += 1;
                }
            });
        }

        // COM 側の成駒をカウント。
        leaf_eval.com_promo_count = (self.pos.bb_occupied_side(COM)
            & (self.pos.bb_piece_kind(PRO_PAWN)
                | self.pos.bb_piece_kind(PRO_LANCE)
                | self.pos.bb_piece_kind(PRO_KNIGHT)
                | self.pos.bb_piece_kind(PRO_SILVER)
                | self.pos.bb_piece_kind(HORSE)
                | self.pos.bb_piece_kind(DRAGON)))
        .count_ones() as u8;

        // COM が自殺手を指したかどうかを利きを見て判定。
        leaf_eval.is_suicide = self.pos.is_checked(COM);

        Some(leaf_eval)
    }

    /// 様々な要素を勘案して末端局面の評価を修正する。
    #[inline]
    fn revise_leaf_evaluation(
        &self,
        root_eval: &RootEvaluation,
        umv: UndoableMove,
        leaf_eval: &mut LeafEvaluation,
    ) {
        let hum_king_sq = root_eval.king_sq[HUM];
        let com_king_sq = root_eval.king_sq[COM];

        let dst_to_com_king = umv.dst().distance(com_king_sq);

        // 指し手の移動先の駒種。駒打ちかどうかを問わない。
        let pk_dst = if umv.is_drop() {
            umv.dropped_piece_kind()
        } else {
            umv.piece_dst().kind()
        };

        // COM 側の玉、龍、馬が取られず、歩(不成)で駒を取る手の評価を上げる。
        if leaf_eval.disadv_price < 20 && leaf_eval.capture_price > 0 && pk_dst == PAWN {
            log_revise_capture_by_pawn(leaf_eval);
            leaf_eval.score_nega.wrapping_sub_assign(1);
        }

        // verify の都合上、ここでの末端局面評価を初期評価としてログ出力する。
        log_leaf_evaluation_ini(leaf_eval);

        // HUM 側の垂れ歩または垂れ香が存在すれば評価を下げる。
        if leaf_eval.hum_hanging {
            log_revise_hum_hanging(leaf_eval);
            leaf_eval.score_nega.wrapping_add_assign(4);
        }

        // 中盤以降は COM 玉から遠い歩またはと金を取られるのを軽視する。
        if (root_eval.power_hum >= 15 || root_eval.power_com >= 15) && leaf_eval.score_nega < 3 {
            // 駒損マスがなければ leaf_eval.disadv_price は 0 なので、何もする必要がない。
            if let Some(disadv_sq) = leaf_eval.disadv_sq {
                if com_king_sq.distance(disadv_sq) >= 4 {
                    log_revise_midgame_attacked_pawn(leaf_eval);
                    leaf_eval
                        .score_nega
                        .wrapping_sub_assign(leaf_eval.disadv_price);
                }
            }
        }

        // 終盤用
        if root_eval.power_hum >= 25 || root_eval.power_com >= 25 {
            // 互いの玉から遠い最大駒得マスを軽視する。
            if let Some(adv_sq) = leaf_eval.adv_sq {
                if hum_king_sq.distance(adv_sq) >= 4 && com_king_sq.distance(adv_sq) >= 3 {
                    log_revise_endgame_unimportant_adv_sq(leaf_eval);
                    leaf_eval
                        .score_posi
                        .wrapping_sub_assign(leaf_eval.adv_price);
                }
            }

            // 互いの玉から遠い安い駒を取られるのを軽視する。
            if let Some(disadv_sq) = leaf_eval.disadv_sq {
                if leaf_eval.disadv_price < 7
                    && hum_king_sq.distance(disadv_sq) >= 3
                    && com_king_sq.distance(disadv_sq) >= 3
                {
                    log_revise_endgame_unimportant_cheap_disadv_sq(leaf_eval);
                    leaf_eval
                        .score_nega
                        .wrapping_sub_assign(leaf_eval.disadv_price);
                }
            }

            // 駒取りの評価修正。
            if leaf_eval.capture_price > 0 {
                if leaf_eval.dst_to_hum_king <= 2 {
                    // HUM 玉に近い駒を取る手の評価を上げる。
                    log_revise_endgame_capture_near_hum_king(leaf_eval);
                    leaf_eval.capture_price.wrapping_add_assign(2);
                } else if leaf_eval.dst_to_hum_king >= 4 && dst_to_com_king >= 4 {
                    // 互いの玉から遠い駒を取る手の評価を下げる。
                    log_revise_endgame_unimportant_capture(leaf_eval);
                    leaf_eval.capture_price.wrapping_sub_assign(3);
                }
            }
        }

        // 寄せが見込めない状況ではむやみに王手をかけない。
        // ただし「王手xx取り」を除く。
        if leaf_eval.adv_price >= 30
            && leaf_eval.hum_king_threat_around25 < 12
            && root_eval.rbp_com < 4
            && root_eval.power_com < 35
            && leaf_eval.score_posi.wrapping_sub(leaf_eval.adv_price) < 3
        {
            log_revise_useless_check(leaf_eval);
            leaf_eval
                .score_posi
                .wrapping_sub_assign(leaf_eval.adv_price);
        }

        // 高い駒を自陣側かつ互いの玉から遠くに打つ手の評価を下げる(合駒は除く)。
        if umv.is_drop()
            && (SILVER <= pk_dst && pk_dst <= GOLD)
            && umv.dst().row() <= ROW_5
            && root_eval.disadv_price < 30
            && leaf_eval.dst_to_hum_king >= 3
            && dst_to_com_king >= 3
        {
            log_revise_useless_drop(leaf_eval);
            leaf_eval.score_nega.wrapping_add_assign(2);
        }

        // 意図がよくわからない。手駒が多いと駒取りをより高く評価する...?
        if root_eval.power_com >= 27 {
            if leaf_eval.score_posi >= 6 {
                log_revise_increase_capture_price(leaf_eval);
                leaf_eval.capture_price.wrapping_add_assign(4);
            } else if leaf_eval.score_posi >= 3 {
                log_revise_increase_capture_price(leaf_eval);
                leaf_eval.capture_price.wrapping_add_assign(1);
            }
        }

        // 大駒を打つ手は敵陣側ほど評価を高くする(合駒の場合はペナルティなし)。
        if umv.is_drop() && (pk_dst == BISHOP || pk_dst == ROOK) {
            let row = umv.dst().row();
            if row >= ROW_8 {
                log_revise_good_rook_bishop_drop(leaf_eval);
                leaf_eval.score_posi.wrapping_add_assign(2);
                leaf_eval.score_nega.wrapping_sub_assign(2);
            } else if root_eval.disadv_price < 30 {
                log_revise_bad_rook_bishop_drop(leaf_eval);
                leaf_eval.score_posi.wrapping_sub_assign(2);
                leaf_eval.score_nega.wrapping_add_assign(2);
                if row <= ROW_4 {
                    leaf_eval.score_nega.wrapping_add_assign(2);
                }
            }
        }

        // 玉で駒を取る手は評価を下げる(なるべく他の駒で取る)。
        //
        // XXX: これ、駒取りでなくても減算が行われてしまう。
        if pk_dst == KING {
            log_revise_capture_by_king(leaf_eval);
            leaf_eval.capture_price.wrapping_sub_assign(1);
            leaf_eval.score_posi.wrapping_sub_assign(2);
        }

        // 意図がよくわからない。
        // 最後の条件は本来 com_king_sq でなく hum_king_sq だったのかも。
        // だとすれば、「戦力が豊富で HUM 玉に十分迫れていれば、駒損しない限り
        // HUM 玉周辺の駒得マスは対象が安い駒でも評価を上げる」と解釈できる。
        //
        // XXX: ここでは駒得マスが存在しなくても評価修正が行われる。
        if root_eval.power_com >= 31
            && leaf_eval.adv_price < 4
            && leaf_eval.disadv_price == 0
            && leaf_eval.hum_king_threat_around25 >= 7
            && naitou_square_distance(leaf_eval.adv_sq, com_king_sq) <= 2
        {
            log_revise_cheap_adv_sq_near_hum_king(leaf_eval);
            let bonus = (leaf_eval.hum_king_threat_around25 - 7) / 2;
            leaf_eval.score_posi.wrapping_add_assign(bonus);
        }

        // 自分から角をぶつける手(駒打ちかどうかによらず)を抑制する。
        if leaf_eval.adv_price == 16 && pk_dst == BISHOP {
            log_revise_inhibit_bishop_exchange(leaf_eval);
            leaf_eval
                .score_posi
                .wrapping_sub_assign(leaf_eval.adv_price);
            leaf_eval.adv_price = 0;
        }

        // 戦力が豊富なとき、手駒の飛車角を温存する手は COM 玉の危険度が高いほど評価を下げる。
        if root_eval.power_com >= 27 && !(umv.is_drop() && (pk_dst == BISHOP || pk_dst == ROOK)) {
            log_revise_keep_rook_bishop_in_emergency(leaf_eval);
            let penalty = 4 * leaf_eval.com_king_choke_count_around8;
            leaf_eval.score_posi.wrapping_sub_assign(penalty);
            leaf_eval.score_nega.wrapping_add_assign(penalty);
        }

        // 優勢なときは高い駒を取りながら HUM 玉に迫る手の評価を上げ、駒損を軽視する。
        if leaf_eval.capture_price >= 8
            && (H_SILVER <= umv.piece_captured() && umv.piece_captured() <= H_KING)
            && (leaf_eval.adv_price >= 30
                || naitou_square_distance(leaf_eval.adv_sq, hum_king_sq) < 3)
            && root_eval.power_com >= 30
            && leaf_eval.hum_king_threat_around25 >= 7
            && root_eval.rbp_com >= 4
        {
            log_revise_capture_near_hum_king(leaf_eval);
            leaf_eval.score_posi.wrapping_add_assign(2);
            if (8..30).contains(&leaf_eval.disadv_price) {
                leaf_eval.score_nega = 8;
                leaf_eval.disadv_price = 8;
            }
        }

        // COM 玉が危険な場合、玉で駒を取るのは価値なしとする。
        //
        // XXX: この部分、原作では候補手が駒打ちのとき配列外参照が起こり、
        // 参照先が COM 玉を表す値だと条件が真になってしまう。
        // この挙動を模倣するのは難しいので諦めた。レアケースなのでそこまで問題にはならないはず。
        if leaf_eval.com_king_threat_around8 >= 5 && pk_dst == KING {
            log_revise_capture_by_king_in_emergency(leaf_eval);
            leaf_eval.capture_price = 0;
        }

        // 戦力が豊富なら駒を取りつつ王手する手の評価を上げる。
        if root_eval.power_com >= 35 && leaf_eval.adv_price >= 30 && leaf_eval.capture_price >= 2 {
            log_revise_capturing_check(leaf_eval);
            leaf_eval.score_nega.wrapping_sub_assign(2);
        }

        // ある程度戦力があるとき、capture_price が小さくても score_posi に応じて水増しする。
        // (意図がよくわからず。次に駒を取る手も選択肢に含めたいということ?)
        if root_eval.power_com >= 20 && leaf_eval.capture_price < 2 {
            if leaf_eval.score_posi >= 5 {
                log_revise_cheap_capture_price(leaf_eval);
            }
            match leaf_eval.score_posi {
                5..=9 => leaf_eval.capture_price.wrapping_add_assign(1),
                10..=19 => leaf_eval.capture_price.wrapping_add_assign(2),
                20.. => leaf_eval.capture_price.wrapping_add_assign(3),
                _ => {}
            }
        }

        // 大駒を敵陣以外に打つ手の評価を下げる。
        if umv.is_drop() && (pk_dst == BISHOP || pk_dst == ROOK) && umv.dst().row() <= ROW_6 {
            log_revise_bad_rook_bishop_drop_2(leaf_eval);
            leaf_eval.score_posi.wrapping_sub_assign(3);
            leaf_eval.score_nega.wrapping_add_assign(3);
        }

        // 成駒を動かす場合、HUM 玉に近づく手の方を高く評価する。
        if !umv.is_drop() && umv.piece_src().is_promoted() {
            log_revise_promoted_walk(leaf_eval);
            let value = umv
                .src()
                .distance(hum_king_sq)
                .wrapping_sub(umv.dst().distance(hum_king_sq));
            leaf_eval.score_posi.wrapping_add_assign(value);
        }

        // 戦力が豊富なら王手の評価を上げる。
        if root_eval.power_com >= 25 && leaf_eval.adv_price >= 30 {
            log_revise_check_with_power(leaf_eval);
            leaf_eval.score_posi.wrapping_add_assign(4);
            leaf_eval.capture_price.wrapping_add_assign(1);
            leaf_eval.score_nega.wrapping_sub_assign(2);
        }

        // 高い駒を取りながらの王手の評価を上げる。
        if leaf_eval.adv_price >= 30 && leaf_eval.capture_price >= 8 {
            log_revise_good_capturing_check(leaf_eval);
            leaf_eval.score_nega.wrapping_sub_assign(4);
        }

        // capture_price, score_posi, score_nega については負なら 0 とする。
        #[inline]
        fn saturate_negative(x: &mut u8) {
            if *x & 0x80 != 0 {
                *x = 0;
            }
        }
        saturate_negative(&mut leaf_eval.capture_price);
        saturate_negative(&mut leaf_eval.score_posi);
        saturate_negative(&mut leaf_eval.score_nega);
    }

    /// 候補手が現在の最善手より優れているかどうかを返す。
    #[inline]
    fn can_improve_best(
        &self,
        root_eval: &RootEvaluation,
        best_eval: &LeafEvaluation,
        leaf_eval: &LeafEvaluation,
        umv: UndoableMove,
    ) -> bool {
        /// タイブレーク処理。
        ///
        /// * `lhs > rhs` ならば `true` を返す。
        /// * `lhs < rhs` ならば `false` を返す。
        /// * `lhs == rhs` ならば何もせず次の処理に移る。
        macro_rules! tie_break_with_log {
            ($lhs:expr, $rhs:expr, $f_log:expr) => {{
                match $lhs.cmp(&$rhs) {
                    Ordering::Greater => {
                        $f_log(true);
                        return true;
                    }
                    Ordering::Less => {
                        $f_log(false);
                        return false;
                    }
                    Ordering::Equal => {}
                }
            }};
        }

        // 候補手が自殺手で最善手が自殺手でないなら、明らかに最善手を採用すべき。
        //
        // XXX: ここ、閾値がギリギリなので、取り返しフラグによる補正が入ると自殺手を弾けないことがある。
        // 後の最善手更新のされ方によっては COM 玉が詰んでないのに投了することもありうる。
        if leaf_eval.disadv_price >= 40 && best_eval.disadv_price < 40 {
            log_cmp_suicide(false);
            return false;
        }

        // 候補手が自殺手でなく最善手が自殺手なら、明らかに候補手を採用すべき。
        if leaf_eval.disadv_price < 40 && best_eval.disadv_price >= 40 {
            log_cmp_suicide(true);
            return true;
        }

        // まず score_nega を比較する。
        //
        // score_posi は基本的には「次に取れる駒」の価値の総和であり、
        // score_nega は基本的には「今取られる駒」の価値の総和だから、
        // 基本的に後者の方が重要と考えられる。
        //
        // ただし、中盤以降は駒取りを手抜いて攻め合うこともよくあるので、
        // そのような状況では score_posi を考慮する余地もあると考えられる。
        match leaf_eval.score_nega.cmp(&best_eval.score_nega) {
            // 候補手の方が score_nega が悪い場合、capture_price を比較する。
            // このケースではタイブレークは発生しない。
            Ordering::Greater => match leaf_eval.capture_price.cmp(&best_eval.capture_price) {
                // 候補手の方が capture_price も悪いなら、明らかに最善手を採用すべき。
                Ordering::Less => {
                    log_cmp_nega_worse_capture_price_worse();
                    return false;
                }

                // 候補手の方が capture_price が良いなら、
                // その差分が score_nega の差分以上であれば候補手を採用する。
                Ordering::Greater => {
                    let dcapture = leaf_eval.capture_price - best_eval.capture_price;
                    let dnega = leaf_eval.score_nega - best_eval.score_nega;
                    let improved = dcapture >= dnega;
                    log_cmp_nega_worse_capture_price_better(improved);
                    return improved;
                }

                // capture_price が等しい場合、大抵は最善手の方が良いと考えられる。
                // 限られたケースでのみ候補手を採用する。
                Ordering::Equal => {
                    // ある程度戦力があり、候補手も最善手も駒取りでなく、候補手の方が score_posi が良いとき、
                    // score_posi の差分が score_nega の差分より大きければ候補手を採用する。
                    let improved = if root_eval.power_com >= 18
                        && leaf_eval.capture_price == 0
                        && leaf_eval.score_posi > best_eval.score_posi
                    {
                        let dposi = leaf_eval.score_posi - best_eval.score_posi;
                        let dnega = leaf_eval.score_nega - best_eval.score_nega;
                        dposi > dnega
                    } else {
                        false
                    };
                    log_cmp_nega_worse_capture_price_equal(improved);
                    return improved;
                }
            },

            // 候補手の方が score_nega が良い場合、基本的には候補手の方が良いと考えられる。
            // このケースではタイブレークが発生しうる。
            Ordering::Less => {
                // 最善手の score_nega が大きすぎるなら直ちに候補手を採用する。
                if (30..80).contains(&best_eval.score_nega) {
                    log_cmp_nega_better_extreme();
                    return true;
                }

                // capture_price を比較。
                match leaf_eval.capture_price.cmp(&best_eval.capture_price) {
                    // 候補手の方が capture_price も良いなら、明らかに候補手を採用すべき。
                    Ordering::Greater => {
                        log_cmp_nega_better_capture_price_better();
                        return true;
                    }

                    // 候補手の方が capture_price が悪い場合、その差分を score_nega の差分と比較して決める。
                    // 差分が等しい場合、タイブレークとする。
                    Ordering::Less => {
                        let dcapture = best_eval.capture_price - leaf_eval.capture_price;
                        let dnega = best_eval.score_nega - leaf_eval.score_nega;
                        tie_break_with_log!(
                            dnega,
                            dcapture,
                            log_cmp_nega_better_capture_price_worse
                        );
                    }

                    // capture_price が等しい場合、大抵は候補手の方が良いと考えられる。
                    // 限られたケースでのみ最善手を採用する。
                    Ordering::Equal => {
                        // ある程度戦力があり、候補手も最善手も駒取りでなく、候補手の方が score_posi が悪いとき、
                        // score_posi の差分を score_nega の差分と比較して決める。
                        // 差分が等しい場合、タイブレークとする。
                        if root_eval.power_com >= 18
                            && leaf_eval.capture_price == 0
                            && leaf_eval.score_posi < best_eval.score_posi
                        {
                            let dposi = best_eval.score_posi - leaf_eval.score_posi;
                            let dnega = best_eval.score_nega - leaf_eval.score_nega;
                            tie_break_with_log!(
                                dnega,
                                dposi,
                                log_cmp_nega_better_capture_price_equal
                            );
                        } else {
                            log_cmp_nega_better_capture_price_equal(true);
                            return true;
                        }
                    }
                }
            }

            // score_nega が等しい場合、capture_price が良い方を採用する。等しければタイブレーク。
            Ordering::Equal => {
                tie_break_with_log!(
                    leaf_eval.capture_price,
                    best_eval.capture_price,
                    log_cmp_nega_equal
                );
            }
        }

        // タイブレーク処理。

        // 成駒の個数が異なるなら、多い方を採用。
        tie_break_with_log!(
            leaf_eval.com_promo_count,
            best_eval.com_promo_count,
            log_cmp_com_promo_count
        );

        // score_posi が異なるなら、良い方を採用。
        tie_break_with_log!(
            leaf_eval.score_posi,
            best_eval.score_posi,
            log_cmp_score_posi
        );

        // adv_price が異なるなら、良い方を採用。
        tie_break_with_log!(leaf_eval.adv_price, best_eval.adv_price, log_cmp_adv_price);

        if umv.is_drop() {
            // 合駒でない限り、駒打ちより盤上の駒を動かす手を優先する。
            if root_eval.disadv_price < 30 {
                log_cmp_prefer_walk();
                return false;
            }
            // 合駒の場合、より安い駒を打つ手なら採用する。
            //
            // XXX: naitou_best_src_value は局面ごとに初期化されないため、
            // 場合によっては以前の局面に影響されることもありうる。
            let value = naitou_com_drop_src_value(umv.dropped_piece_kind());
            let improved = value < self.naitou_best_src_value;
            if improved {
                log_cmp_drop_prefer_cheap();
            }
            improved
        } else {
            // 盤上の駒を動かす手同士を雑多な項目で比較する。

            // HUM 玉の危険度に差があるなら、良い方を採用。
            tie_break_with_log!(
                leaf_eval.hum_king_threat_around25,
                best_eval.hum_king_threat_around25,
                log_cmp_walk_hum_king_threat_around25
            );

            // COM 玉の危険度に差があるなら、良い方を採用。
            tie_break_with_log!(
                leaf_eval.com_king_safety_around25,
                best_eval.com_king_safety_around25,
                log_cmp_walk_com_king_safety_around25
            );
            tie_break_with_log!(
                best_eval.com_king_threat_around25,
                leaf_eval.com_king_threat_around25,
                log_cmp_walk_com_king_threat_around25
            );

            // 離れ駒の個数が異なるなら、少ない方を採用。
            tie_break_with_log!(
                best_eval.com_loose_count,
                leaf_eval.com_loose_count,
                log_cmp_walk_com_loose_count
            );

            // COM 玉から遠い駒を動かす候補手の場合、移動先が最善手より HUM 玉に近ければ採用。
            if leaf_eval.src_to_com_king >= 3 {
                tie_break_with_log!(
                    best_eval.dst_to_hum_king,
                    leaf_eval.dst_to_hum_king,
                    log_cmp_walk_dst_to_hum_king
                );
            }

            // 候補手の移動元が最善手より COM 玉から遠ければ採用。
            // 自玉周りの駒はなるべく動かさない意図か。
            let improved = leaf_eval.src_to_com_king > best_eval.src_to_com_king;
            log_cmp_walk_src_to_com_king(improved);
            improved
        }
    }

    /// 定跡処理。定跡手が存在し、かつ採用可能ならそれを返す。
    #[inline]
    fn think_book(&mut self, mv_hum: Option<Move>) -> Option<Move> {
        log_book_start();

        // いずれかの定跡手が採用されるか、もしくは定跡手が尽きるまでループ。
        // 却下された定跡手も book_state からは捨てられることに注意。
        loop {
            // 定跡手を取得。定跡手が尽きたら終了。
            let book_mv = self.book_state.next_move(&self.pos, self.progress_ply)?;

            // 違法手は却下。
            if !self.book_move_is_legal(book_mv) {
                continue;
            }

            // 移動先の利き数が勝っていなければ却下。
            if self.pos.effect_count_board(HUM)[book_mv.dst()]
                >= self.pos.effect_count_board(COM)[book_mv.dst()]
            {
                continue;
            }

            // 定跡手を指した局面を評価し、駒損するなら原則として却下。
            // ただし、progress_ply <= 6 かつ直前の HUM の指し手の移動先が４五の場合のみ却下しない。
            // これは裏技的要素と思われる(いきなり右桂を跳ね出して５三を破る手が成立する)。
            let disadv_price = self.evaluate_book_move(book_mv);
            if disadv_price > 0
                && !(self.progress_ply <= 6 && mv_hum.map_or(false, |mv| mv.dst() == SQ_45))
            {
                continue;
            }

            // 全てのチェックを通ったら定跡手を採用。
            log_book_accept_move(book_mv);
            return Some(book_mv);
        }
    }

    /// 定跡手が合法かどうかを返す。
    #[inline]
    fn book_move_is_legal(&self, mv: Move) -> bool {
        // 定跡手に駒打ちは含まれない。
        debug_assert!(!mv.is_drop());

        // 定跡手は全て不成である。
        debug_assert!(!mv.is_promotion());

        // 定跡手で行きどころのない駒が生じることはない。
        debug_assert!(mv.dst().row() <= ROW_7);

        // 移動先に COM 駒があってはならない。
        let pc_dst = self.pos.board()[mv.dst()];
        if pc_dst != NO_PIECE && pc_dst.side() == COM {
            return false;
        }

        // 移動元には COM 駒がなければならない。
        let pc_src = self.pos.board()[mv.src()];
        if pc_src == NO_PIECE || pc_src.side() != COM {
            return false;
        }

        // 移動先は移動元の COM 駒の利きがあるマスでなければならない。
        if !bbs::effect(pc_src, mv.src(), self.pos.bb_occupied()).test_square(mv.dst()) {
            return false;
        }

        // 玉を動かす場合、移動先に HUM の利きがあってはならない。
        // (これだけでは自殺手の可能性を排除しきれないが、後に駒損する手を弾くので大丈夫、なはず)
        if pc_src == C_KING && self.pos.effect_count_board(HUM)[mv.dst()] != 0 {
            return false;
        }

        true
    }

    /// 定跡手を指した局面を評価し、disadv_price を返す。
    #[inline]
    fn evaluate_book_move(&mut self, mv: Move) -> u8 {
        let umv = self.pos.do_move(mv);

        // 通常の局面評価と同様に disadv_price を求める。
        let mut disadv_price = 0;
        for (_, pk, exchange) in self.iter_disadvantage_squares() {
            util::chmax(&mut disadv_price, naitou_piece_price_d(pk));
            if exchange {
                disadv_price -= 1;
            }
        }

        self.pos.undo_move(umv);

        disadv_price
    }

    /// 全ての駒得マスとその上の HUM 駒種を原作準拠で昇順に列挙する。
    fn iter_advantage_squares(&self) -> impl Iterator<Item = (Square, PieceKind)> + '_ {
        naitou_squares().filter_map(|sq| {
            // HUM 駒がなければ駒得マスではない。
            let pc = self.pos.board()[sq];
            if pc == NO_PIECE || pc.side() != HUM {
                return None;
            }
            let pk = pc.kind();

            // COM の利きがなければ駒得マスではない。
            let eff_com = self.pos.effect_count_board(COM)[sq];
            if eff_com == 0 {
                return None;
            }

            // COM の利きがあり、HUM の利きがなければ駒得マス。
            let eff_hum = self.pos.effect_count_board(HUM)[sq];
            if eff_hum == 0 {
                return Some((sq, pk));
            }

            // 両者の利きがある場合、HUM 駒と COM attacker の価値比較、および進行度で判定する。

            let atk_com = naitou_attacker(&self.pos, COM, sq);
            // 利きがあるなら attacker があるはず。
            debug_assert_ne!(atk_com, NO_PIECE_KIND);

            // (COM attacker の価値) < (HUM 駒の価値) ならば駒得マス。
            // (COM attacker の価値) == (HUM 駒の価値) の場合、(進行度) != 0 ならば駒得マス。
            //
            // XXX: 後者の判定は HUM 側が玉で王手できる不具合の原因となっている。
            // 本プログラムではそもそも玉での王手は生成しないが。
            let price_pc_hum = naitou_piece_price_b(pk);
            let price_atk_com = naitou_piece_price_b(atk_com);
            (price_atk_com < price_pc_hum
                || (price_atk_com == price_pc_hum && self.progress_level != 0))
                .then(|| (sq, pk))
        })
    }

    /// 全ての駒損マスとその上の COM 駒種、および取り返しフラグを原作準拠で昇順に列挙する。
    fn iter_disadvantage_squares(&self) -> impl Iterator<Item = (Square, PieceKind, bool)> + '_ {
        // 取り返しフラグ。一度真になるとその後全ての駒損マスにおいて真となる。
        let mut exchange = false;

        naitou_squares().filter_map(move |sq| {
            // COM 駒がなければ駒損マスではない。
            let pc = self.pos.board()[sq];
            if pc == NO_PIECE || pc.side() != COM {
                return None;
            }
            let pk = pc.kind();

            // HUM の利きがなければ駒損マスではない。
            let eff_hum = self.pos.effect_count_board(HUM)[sq];
            if eff_hum == 0 {
                return None;
            }

            // HUM の利きがあり、マス上の COM 駒が玉ならば駒損マス(王手)。
            if pk == KING {
                return Some((sq, pk, exchange));
            }

            // HUM の利きがあり、COM の利きがなければ駒損マス。
            let eff_com = self.pos.effect_count_board(COM)[sq];
            if eff_com == 0 {
                return Some((sq, pk, exchange));
            }

            // 両者の利きがある場合、利き数比較および
            // COM 駒、HUM attacker, COM attacker の価値比較で判定する。

            let atk_hum = naitou_attacker(&self.pos, HUM, sq);
            let atk_com = naitou_attacker(&self.pos, COM, sq);
            // 利きがあるなら attacker があるはず。
            debug_assert_ne!(atk_hum, NO_PIECE_KIND);
            debug_assert_ne!(atk_com, NO_PIECE_KIND);

            let price_pc_com = naitou_piece_price_d(pk);
            let price_atk_hum = naitou_piece_price_c(atk_hum);
            let price_atk_com = naitou_piece_price_d(atk_com);

            let disadv = if eff_com < eff_hum {
                // (COM 利き数) < (HUM 利き数) の場合、
                // COM 側の駒と attacker の価値の和が HUM attacker の価値以上なら駒損マス。
                price_pc_com + price_atk_com >= price_atk_hum
            } else {
                // (COM 利き数) >= (HUM 利き数) の場合、
                // (COM 駒の価値) > (HUM attacker の価値) ならば駒損マスとするが、
                // この場合利きが同数以上で取り返しが利くため、取り返しフラグを立てる。
                if price_pc_com > price_atk_hum {
                    exchange = true;
                    true
                } else {
                    false
                }
            };

            disadv.then(|| (sq, pk, exchange))
        })
    }

    /// `do_step()` を undo し、元の状態を復元する。
    pub fn undo_step(&mut self, resp: &EngineResponse) {
        // 応答が COM の指し手を含むならそれを undo する。
        if let Some(umv_com) = resp.move_com() {
            debug_assert_eq!(self.pos.side_to_move(), HUM);
            self.pos.undo_move(umv_com);
        }

        // HUM の指し手を undo し、全ての状態を復元する。
        debug_assert_eq!(self.pos.side_to_move(), COM);
        let undo_info = resp.undo_info();
        self.pos.undo_move(undo_info.umv_hum);
        self.progress_ply = undo_info.progress_ply;
        self.progress_level = undo_info.progress_level;
        self.progress_level_sub = undo_info.progress_level_sub;
        self.book_state = undo_info.book_state;
        self.naitou_best_src_value = undo_info.naitou_best_src_value;
    }

    /// HUM 側の指し手で局面を進め、内部状態を更新し、`EngineUndoInfo` を返す。
    ///
    /// `mv` が自殺手の場合、エラーを返す。
    #[inline]
    fn do_move_hum(&mut self, mv: Move) -> anyhow::Result<EngineUndoInfo> {
        debug_assert_eq!(self.pos.side_to_move(), HUM);

        // HUM 側の着手を行い、UndoableMove を取得。
        let umv_hum = self.pos.do_move(mv);

        // mv が自殺手だった場合、局面を元に戻してエラーを返す。
        if self.pos.is_checked(HUM) {
            self.pos.undo_move(umv_hum);
            bail!("suicide move");
        }

        // undo 用情報を取得しておく。
        let progress_ply = self.progress_ply;
        let progress_level = self.progress_level;
        let progress_level_sub = self.progress_level_sub;
        let book_state = self.book_state;
        let naitou_best_src_value = self.naitou_best_src_value;

        // 進行度更新。
        self.increment_progress_ply();
        if self.progress_ply >= 51 {
            self.progress_level = (self.progress_level + 1).min(2);
        }
        if self.progress_ply >= 71 {
            self.progress_level = 3;
        }

        Ok(EngineUndoInfo {
            umv_hum,
            progress_ply,
            progress_level,
            progress_level_sub,
            book_state,
            naitou_best_src_value,
        })
    }

    /// COM 側の指し手で局面を進め、内部状態を更新し、`UndoableMove` を返す。
    #[inline]
    fn do_move_com(&mut self, mv: Move) -> UndoableMove {
        debug_assert_eq!(self.pos.side_to_move(), COM);

        // COM 側の着手を行い、UndoableMove を取得。
        let umv_com = self.pos.do_move(mv);

        // 進行度更新。COM 側は単に手数をインクリメントするだけ。
        self.increment_progress_ply();

        umv_com
    }

    /// 進行度管理用の手数をインクリメントする(最大 100)。
    #[inline]
    fn increment_progress_ply(&mut self) {
        self.progress_ply = (self.progress_ply + 1).min(100);
    }
}
