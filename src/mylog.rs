//! 思考ログ出力。

use log::info;

use crate::book::Formation;
use crate::effect::EffectCountBoard;
use crate::engine::{LeafEvaluation, RootEvaluation};
use crate::shogi::*;

/// 思考開始ログを出力する。
pub fn log_think_start(ply: u32) {
    info!(
        "# ------------------------------ {} 手目 思考開始 ------------------------------ {{{{{{",
        ply
    );
}

/// 思考終了ログを出力する。
pub fn log_think_end() {
    info!("# ------------------------------ 思考終了 ------------------------------ }}}}}}");
    info!("");
}

/// 候補手評価開始ログを出力する。
pub fn log_cand_start(mv: Move) {
    info!(
        "## ------------------------------ 候補手評価開始: {} ------------------------------ {{{{{{",
        mv
    );
    info!("");
}

/// 候補手が駒捨てを理由に却下されたログを出力する。
pub fn log_cand_reject_by_sacrifice() {
    info!("候補手却下 (駒捨て)");
    info!("");
}

/// 候補手が打ち歩詰めを理由に却下されたログを出力する。
pub fn log_cand_reject_by_drop_pawn_mate() {
    info!("候補手却下 (打ち歩詰め)");
    info!("");
}

/// 候補手評価終了ログを出力する。
pub fn log_cand_end() {
    info!("## ------------------------------ 候補手評価終了 ------------------------------ }}}}}}");
    info!("");
}

/// 与えられた局面をログ出力する。
pub fn log_position(side_to_move: Side, board: &Board, hands: &Hands) {
    info!("手番: {}", side_to_move);
    info!("");
    info!("COM 手駒: {}", hands[COM]);
    info!("");
    info!("{}", board);
    info!("HUM 手駒: {}", hands[HUM]);
    info!("");
}

/// 与えられた盤面をログ出力する。
///
/// 原作では末端局面については手駒が更新されないため、盤面のみをログ出力したいことがある。
pub fn log_board(board: &Board) {
    info!("{}", board);
}

/// 与えられた陣営とその `EffectCountBoard` をログ出力する。
pub fn log_effect_count_board(side: Side, ecb: &EffectCountBoard) {
    info!("{} 利き数:", side);
    info!("{}", ecb);
}

/// 進行度関連情報をログ出力する。
pub fn log_progress(progress_ply: u8, progress_level: u8, progress_level_sub: u8) {
    info!("進行度手数: {}", progress_ply);
    info!("進行度: {}", progress_level);
    info!("サブ進行度: {}", progress_level_sub);
    info!("");
}

/// 現在の戦型をログ出力する。
pub fn log_formation(formation: Formation) {
    info!("現在の戦型: {}", formation);
    info!("");
}

/// 思考エンジンの通常の指し手応答をログ出力する。
pub fn log_engine_response_move(mv: Move) {
    info!("COM 指し手: {}", mv);
}

/// 思考エンジンの HUM 勝ち応答をログ出力する。
pub fn log_engine_response_hum_win() {
    info!("HUM 勝ち");
}

/// 思考エンジンの HUM 自殺手応答をログ出力する。
pub fn log_engine_response_hum_suicide() {
    info!("HUM 自殺手");
}

/// 思考エンジンの COM 勝ち応答をログ出力する。
pub fn log_engine_response_com_win(mv: Move) {
    info!("COM 勝ち: {}", mv);
}

/// ルート局面の評価をログ出力する。
pub fn log_root_evaluation(root_eval: &RootEvaluation) {
    info!("ルート局面評価:");
    info!("  adv_price: {}", root_eval.adv_price);
    info!("  disadv_price: {}", root_eval.disadv_price);
    info!("  power_hum: {}", root_eval.power_hum);
    info!("  power_com: {}", root_eval.power_com);
    info!("  rbp_com: {}", root_eval.rbp_com);
    info!("");
}

/// 末端局面評価修正: 歩で駒を取る手。
/// verify の都合上、`leaf_eval` には **修正前の** 評価を与えること。他の評価修正項目についても同様。
pub fn log_revise_capture_by_pawn(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 歩で駒を取る手");
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面の初期評価をログ出力する。
///
/// verify の都合上、歩で駒を取る補正がかかった直後に呼ぶこととする。
pub fn log_leaf_evaluation_ini(leaf_eval: &LeafEvaluation) {
    info!("末端局面評価 (初期):");

    log_leaf_evaluation_impl(leaf_eval);
}

/// 末端局面評価修正: HUM 側の垂れ歩または垂れ香の存在。
pub fn log_revise_hum_hanging(leaf_eval: &LeafEvaluation) {
    info!("評価修正: HUM 側の垂れ歩/垂れ香が存在");
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 中盤以降で COM 玉から遠い歩またはと金を取られるのを軽視。
pub fn log_revise_midgame_attacked_pawn(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 中盤以降で COM 玉から遠い歩/と金を取られるのを軽視");
    info!("  disadv_price: {}", leaf_eval.disadv_price);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 終盤で互いの玉から遠い最大駒得マスを軽視。
pub fn log_revise_endgame_unimportant_adv_sq(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 終盤で互いの玉から遠い最大駒得マスを軽視");
    info!("  adv_price: {}", leaf_eval.adv_price);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 終盤で互いの玉から遠い安い駒を取られるのを軽視。
pub fn log_revise_endgame_unimportant_cheap_disadv_sq(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 終盤で互いの玉から遠い安い駒を取られるのを軽視");
    info!("  disadv_price: {}", leaf_eval.disadv_price);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 終盤で HUM 玉に近い駒を取る手の評価を上げる。
pub fn log_revise_endgame_capture_near_hum_king(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 終盤で HUM 玉に近い駒を取る手の評価を上げる");
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("");
}

/// 末端局面評価修正: 終盤で互いの玉から遠い駒を取る手の評価を下げる。
pub fn log_revise_endgame_unimportant_capture(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 終盤で互いの玉から遠い駒を取る手の評価を下げる");
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("");
}

/// 末端局面評価修正: 寄せが見込めない状況ではむやみに王手をかけない。
pub fn log_revise_useless_check(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 寄せが見込めない状況ではむやみに王手をかけない");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 高い駒を自陣側かつ互いの玉から遠くに打つ手の評価を下げる。
pub fn log_revise_useless_drop(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 高い駒を自陣側かつ互いの玉から遠くに打つ手の評価を下げる");
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 手駒が多いと駒取りをより高く評価する?
pub fn log_revise_increase_capture_price(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 手駒が多いと駒取りをより高く評価する(?)");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("");
}

/// 末端局面評価修正: 大駒を 8, 9 段目に打つ手の評価を上げる。
pub fn log_revise_good_rook_bishop_drop(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 大駒を 8, 9 段目に打つ手の評価を上げる");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 大駒を 8, 9 段目以外に打つ手の評価を下げる(合駒は除く)。
pub fn log_revise_bad_rook_bishop_drop(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 大駒を 8, 9 段目以外に打つ手の評価を下げる(合駒は除く)");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 玉で駒を取る手は評価を下げる。
pub fn log_revise_capture_by_king(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 玉で駒を取る手は評価を下げる");
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 特定条件下で HUM 玉周辺の安い最大駒得マスの評価を上げる。
pub fn log_revise_cheap_adv_sq_near_hum_king(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 特定条件下で HUM 玉周辺の安い最大駒得マスの評価を上げる");
    info!(
        "  hum_king_threat_around25: {}",
        leaf_eval.hum_king_threat_around25
    );
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 自分から角をぶつける手を抑制する。
pub fn log_revise_inhibit_bishop_exchange(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 自分から角をぶつける手を抑制する");
    info!("  adv_price: {}", leaf_eval.adv_price);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 戦力が豊富なとき、手駒の飛車角を温存する手は COM 玉の危険度が高いほど評価を下げる。
pub fn log_revise_keep_rook_bishop_in_emergency(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 戦力が豊富なとき、手駒の飛車角を温存する手は COM 玉の危険度が高いほど評価を下げる");
    info!(
        "  com_king_choke_count_around8: {}",
        leaf_eval.com_king_choke_count_around8
    );
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 優勢なときは高い駒を取りながら HUM 玉に迫る手の評価を上げ、駒損を軽視する。
pub fn log_revise_capture_near_hum_king(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 優勢なときは高い駒を取りながら HUM 玉に迫る手の評価を上げ、駒損を軽視する");
    info!("  disadv_price: {}", leaf_eval.disadv_price);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: COM 玉が危険な場合、玉による駒取りは価値なしとする。
pub fn log_revise_capture_by_king_in_emergency(leaf_eval: &LeafEvaluation) {
    info!("評価修正: COM 玉が危険な場合、玉による駒取りは価値なしとする");
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("");
}

/// 末端局面評価修正: 戦力が豊富なら駒を取りつつ王手する手の評価を上げる。
pub fn log_revise_capturing_check(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 戦力が豊富なら駒を取りつつ王手する手の評価を上げる");
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: ある程度戦力があるとき、安い駒取りを score_posi に応じて水増しする。
pub fn log_revise_cheap_capture_price(leaf_eval: &LeafEvaluation) {
    info!("評価修正: ある程度戦力があるとき、安い駒取りを score_posi に応じて水増しする");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("");
}

/// 末端局面評価修正: 大駒を敵陣以外に打つ手の評価を下げる。
pub fn log_revise_bad_rook_bishop_drop_2(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 大駒を敵陣以外に打つ手の評価を下げる");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 成駒を動かす場合、HUM 玉に近づく手の方を高く評価する。
pub fn log_revise_promoted_walk(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 成駒を動かす場合、HUM 玉に近づく手の方を高く評価する");
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("");
}

/// 末端局面評価修正: 戦力が豊富なら王手の評価を上げる。
pub fn log_revise_check_with_power(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 戦力が豊富なら王手の評価を上げる");
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面評価修正: 高い駒を取りながらの王手の評価を上げる。
pub fn log_revise_good_capturing_check(leaf_eval: &LeafEvaluation) {
    info!("評価修正: 高い駒を取りながらの王手の評価を上げる");
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!("");
}

/// 末端局面の修正後評価をログ出力する。
pub fn log_leaf_evaluation_revised(leaf_eval: &LeafEvaluation) {
    info!("末端局面評価 (修正後):");

    log_leaf_evaluation_impl(leaf_eval);
}

/// HUM 玉の詰み判定が出た旨をログ出力する。
pub fn log_hum_is_checkmated() {
    info!("### HUM 玉は詰み");
    info!("");
}

/// 候補手と最善手との比較開始ログを出力する。
pub fn log_cmp_start() {
    info!("### 候補手と最善手の比較");
    info!("");
}

/// 比較: 候補手、最善手のいずれか一方のみが自殺手。
pub fn log_cmp_suicide(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は自殺手でなく、最善手は自殺手");
    } else {
        info!("最善手を採用: 候補手は自殺手で、最善手は自殺手でない");
    }
    info!("");
}

/// 比較: 候補手は `score_nega`, `capture_price` ともに劣るため最善手を採用。
pub fn log_cmp_nega_worse_capture_price_worse() {
    info!("最善手を採用: 候補手は score_nega, capture_price ともに劣る");
    info!("");
}

/// 比較: 候補手は `score_nega` で劣るが `capture_price` は優る。
pub fn log_cmp_nega_worse_capture_price_better(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は score_nega で劣るが capture_price で挽回");
    } else {
        info!("最善手を採用: 候補手は score_nega で劣り、capture_price で挽回できず");
    }
    info!("");
}

/// 比較: 候補手は `score_nega` で劣り、`capture_price` は等しい。
pub fn log_cmp_nega_worse_capture_price_equal(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は score_nega で劣り、capture_price が等しいが、他の採用条件が満たされた");
    } else {
        info!("最善手を採用: 候補手は score_nega で劣り、capture_price が等しく、他の採用条件が満たされなかった");
    }
    info!("");
}

/// 比較: 候補手は `score_nega` で優り、最善手の `score_nega` が極端に大きいため候補手を採用。
pub fn log_cmp_nega_better_extreme() {
    info!("候補手を採用: 候補手は score_nega で優り、最善手の score_nega が極端に大きい");
    info!("");
}

/// 比較: 候補手は `score_nega`, `capture_price` ともに優るため候補手を採用。
pub fn log_cmp_nega_better_capture_price_better() {
    info!("候補手を採用: 候補手は score_nega, capture_price ともに優る");
    info!("");
}

/// 比較: 候補手は `score_nega` で優るが `capture_price` で劣る。
///
/// タイブレークになった場合は呼ばれない。
pub fn log_cmp_nega_better_capture_price_worse(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は score_nega で優り capture_price で劣るが、前者の差分の方が大きい");
    } else {
        info!("最善手を採用: 候補手は score_nega で優り capture_price で劣るが、後者の差分の方が大きい");
    }
    info!("");
}

/// 比較: 候補手は `score_nega` で優り、`capture_price` は等しい。
pub fn log_cmp_nega_better_capture_price_equal(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は score_nega で優り、capture_price が等しく、他の採用条件が満たされた");
    } else {
        info!("最善手を採用: 候補手は score_nega で優り、capture_price が等しいが、他の採用条件が満たされなかった");
    }
    info!("");
}

/// 比較: 候補手は `score_nega` が等しい。capture_price の優劣で判定される。
///
/// タイブレークになった場合は呼ばれない。
pub fn log_cmp_nega_equal(improved: bool) {
    if improved {
        info!("候補手を採用: 候補手は score_nega が等しく、capture_price で優る");
    } else {
        info!("最善手を採用: 候補手は score_nega が等しく、capture_price で劣る");
    }
    info!("");
}

/// 比較: COM 側の成駒の個数で判定。
pub fn log_cmp_com_promo_count(improved: bool) {
    if improved {
        info!("候補手を採用: COM 側の成駒の個数");
    } else {
        info!("最善手を採用: COM 側の成駒の個数");
    }
    info!("");
}

/// 比較: `score_posi` の優劣で判定。
pub fn log_cmp_score_posi(improved: bool) {
    if improved {
        info!("候補手を採用: score_posi の優劣");
    } else {
        info!("最善手を採用: score_posi の優劣");
    }
    info!("");
}

/// 比較: `adv_price` の優劣で判定。
pub fn log_cmp_adv_price(improved: bool) {
    if improved {
        info!("候補手を採用: adv_price の優劣");
    } else {
        info!("最善手を採用: adv_price の優劣");
    }
    info!("");
}

/// 比較: 駒打ちより盤上の駒を動かす手を優先(合駒を除く)。
pub fn log_cmp_prefer_walk() {
    info!("最善手を採用: 駒打ちより盤上の駒を動かす手を優先(合駒を除く)");
    info!("");
}

/// 比較: 合駒を打つ場合、より安い駒を優先。
pub fn log_cmp_drop_prefer_cheap() {
    info!("候補手を採用: 合駒を打つ場合、より安い駒を優先");
    info!("");
}

/// 比較: 盤上の駒を動かす手を `hum_king_threat_around25` の優劣で判定。
pub fn log_cmp_walk_hum_king_threat_around25(improved: bool) {
    if improved {
        info!("候補手を採用: hum_king_threat_around25 の優劣");
    } else {
        info!("最善手を採用: hum_king_threat_around25 の優劣");
    }
    info!("");
}

/// 比較: 盤上の駒を動かす手を `com_king_safety_around25` の優劣で判定。
pub fn log_cmp_walk_com_king_safety_around25(improved: bool) {
    if improved {
        info!("候補手を採用: com_king_safety_around25 の優劣");
    } else {
        info!("最善手を採用: com_king_safety_around25 の優劣");
    }
    info!("");
}

/// 比較: 盤上の駒を動かす手を `com_king_threat_around25` の優劣で判定。
pub fn log_cmp_walk_com_king_threat_around25(improved: bool) {
    if improved {
        info!("候補手を採用: com_king_threat_around25 の優劣");
    } else {
        info!("最善手を採用: com_king_threat_around25 の優劣");
    }
    info!("");
}

/// 比較: 盤上の駒を動かす手を COM 側の離れ駒の個数で判定。
pub fn log_cmp_walk_com_loose_count(improved: bool) {
    if improved {
        info!("候補手を採用: COM 側の離れ駒の個数");
    } else {
        info!("最善手を採用: COM 側の離れ駒の個数");
    }
    info!("");
}

/// 比較: COM 玉から遠い駒を動かす手を移動先から HUM 玉への距離で判定。
pub fn log_cmp_walk_dst_to_hum_king(improved: bool) {
    if improved {
        info!("候補手を採用: COM 玉から遠い駒を動かす場合、移動先が HUM 玉に近い方を優先");
    } else {
        info!("最善手を採用: COM 玉から遠い駒を動かす場合、移動先が HUM 玉に近い方を優先");
    }
    info!("");
}

/// 比較: 盤上の駒を動かす手を移動元から COM 玉への距離で判定。
pub fn log_cmp_walk_src_to_com_king(improved: bool) {
    if improved {
        info!("候補手を採用: 移動元が COM 玉から遠い方を優先");
    } else {
        info!("最善手を採用: 移動元が COM 玉から遠い方を優先");
    }
    info!("");
}

/// 現在の最善手とその評価をログ出力する。
pub fn log_best(best_mv: Option<Move>, best_eval: &LeafEvaluation) {
    info!(
        "現在の最善手: {}",
        best_mv.map_or_else(|| "(なし)".to_owned(), |mv| format!("{}", mv))
    );

    info!("  capture_price: {}", best_eval.capture_price);
    info!("  adv_price: {}", best_eval.adv_price);
    info!("  adv_sq: {:?}", best_eval.adv_sq);
    info!("  disadv_price: {}", best_eval.disadv_price);
    info!("  disadv_sq: {:?}", best_eval.disadv_sq);
    info!("  score_posi: {}", best_eval.score_posi);
    info!("  score_nega: {}", best_eval.score_nega);
    info!(
        "  hum_king_threat_around25: {}",
        best_eval.hum_king_threat_around25
    );
    info!(
        "  com_king_safety_around25: {}",
        best_eval.com_king_safety_around25
    );
    info!(
        "  com_king_threat_around25: {}",
        best_eval.com_king_threat_around25
    );
    info!("  dst_to_hum_king: {}", best_eval.dst_to_hum_king);
    info!("  com_promo_count: {}", best_eval.com_promo_count);
    info!("  com_loose_count: {}", best_eval.com_loose_count);
    info!("");
}

fn log_leaf_evaluation_impl(leaf_eval: &LeafEvaluation) {
    info!("  capture_price: {}", leaf_eval.capture_price);
    info!("  adv_price: {}", leaf_eval.adv_price);
    info!("  adv_sq: {:?}", leaf_eval.adv_sq);
    info!("  disadv_price: {}", leaf_eval.disadv_price);
    info!("  disadv_sq: {:?}", leaf_eval.disadv_sq);
    info!("  score_posi: {}", leaf_eval.score_posi);
    info!("  score_nega: {}", leaf_eval.score_nega);
    info!(
        "  hum_king_threat_around25: {}",
        leaf_eval.hum_king_threat_around25
    );
    info!(
        "  com_king_safety_around25: {}",
        leaf_eval.com_king_safety_around25
    );
    info!(
        "  com_king_threat_around25: {}",
        leaf_eval.com_king_threat_around25
    );
    info!(
        "  com_king_threat_around8: {}",
        leaf_eval.com_king_threat_around8
    );
    info!(
        "  com_king_choke_count_around8: {}",
        leaf_eval.com_king_choke_count_around8
    );
    info!("  dst_to_hum_king: {}", leaf_eval.dst_to_hum_king);
    info!("  hum_hanging: {}", leaf_eval.hum_hanging);
    info!("  com_promo_count: {}", leaf_eval.com_promo_count);
    info!("  com_loose_count: {}", leaf_eval.com_loose_count);
    info!("");
}

/// 定跡: 処理開始。
pub fn log_book_start() {
    info!("## 定跡処理");
    info!("");
}

/// 定跡: 定跡手を採用。
pub fn log_book_accept_move(mv: Move) {
    info!("定跡手を採用: {}", mv);
    info!("");
}
