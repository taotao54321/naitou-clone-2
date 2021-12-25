//! ROM 上の各種アドレス。実行フック用。

/// HUM 側の指し手入力待ちループ開始。
pub const HUM_TURN: u16 = 0xCEFC;

/// 思考ルーチン開始。
///
/// この時点で HUM 側の指し手による進行度更新は済んでいる。
/// また、ルート局面における利き情報が計算済みとなっている。
pub const THINK_START: u16 = 0xEF70;

/// ルート局面の評価が完了した。
pub const THINK_EVALUATED_ROOT: u16 = 0xF03E;

/// 盤上の駒を動かす候補手の処理開始。
///
/// この時点で盤面更新は済んでいる。ただし手駒は評価終了まで更新されない。
/// また、末端局面における利き情報が計算済みとなっている。
pub const THINK_CAND_START_WALK: u16 = 0xF0F2;

/// 駒打ちの候補手の処理開始。
///
/// この時点で盤面更新は済んでいる。ただし手駒は評価終了まで更新されない。
/// また、末端局面における利き情報が計算済みとなっている。
pub const THINK_CAND_START_DROP: u16 = 0xF256;

/// 候補手が駒捨てを理由に却下された。
pub const THINK_CAND_REJECT_BY_SACRIFICE: u16 = 0xF2D8;

/// 候補手が打ち歩詰めを理由に却下された。
pub const THINK_CAND_REJECT_BY_DROP_PAWN_MATE: u16 = 0xF2AB;

/// 末端局面評価修正: 歩で駒を取る手。
pub const THINK_CAND_REVISE_CAPTURE_BY_PAWN: u16 = 0xF2C2;

/// 候補手が却下されず、末端局面の初期評価が完了した。
///
/// verify の都合上、歩で駒を取る際の補正がかかった直後となっている。
pub const THINK_CAND_EVALUATED_INI: u16 = 0xF2DC;

/// 末端局面評価修正: HUM 側の垂れ歩または垂れ香が存在。
pub const THINK_CAND_REVISE_HUM_HANGING: u16 = 0xF2EE;

/// 末端局面評価修正: 中盤以降で COM 玉から遠い歩またはと金を取られるのを軽視。
pub const THINK_CAND_REVISE_MIDGAME_ATTACKED_PAWN: u16 = 0xF31F;

/// 末端局面評価修正: 終盤で互いの玉から遠い最大駒得マスを軽視。
pub const THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_ADV_SQ: u16 = 0xF35A;

/// 末端局面評価修正: 終盤で互いの玉から遠い安い駒を取られるのを軽視。
pub const THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_CHEAP_DISADV_SQ: u16 = 0xF38B;

/// 末端局面評価修正: 終盤で HUM 玉に近い駒を取る手の評価を上げる。
pub const THINK_CAND_REVISE_ENDGAME_CAPTURE_NEAR_HUM_KING: u16 = 0xF3CA;

/// 末端局面評価修正: 終盤で互いの玉から遠い駒を取る手の評価を下げる。
pub const THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_CAPTURE: u16 = 0xF3BA;

/// 末端局面評価修正: 寄せが見込めない状況ではむやみに王手をかけない。
pub const THINK_CAND_REVISE_USELESS_CHECK: u16 = 0xF3F7;

/// 末端局面評価修正: 高い駒を自陣側かつ互いの玉から遠くに打つ手の評価を下げる。
pub const THINK_CAND_REVISE_USELESS_DROP: u16 = 0xF41D;

/// 末端局面評価修正: 手駒が多いと駒取りをより高く評価する?
pub const THINK_CAND_REVISE_INCREASE_CAPTURE_PRICE: u16 = 0xF431;

/// 末端局面評価修正: 大駒を 8, 9 段目に打つ手の評価を上げる。
pub const THINK_CAND_REVISE_GOOD_ROOK_BISHOP_DROP: u16 = 0xF4C7;

/// 末端局面評価修正: 大駒を 8, 9 段目に打つ手の評価を下げる(合駒は除く)。
pub const THINK_CAND_REVISE_BAD_ROOK_BISHOP_DROP: u16 = 0xF456;

/// 末端局面評価修正: 玉で駒を取る手は評価を下げる。
pub const THINK_CAND_REVISE_CAPTURE_BY_KING: u16 = 0xF483;

/// 末端局面評価修正: 特定条件下で HUM 玉周辺の安い最大駒得マスの評価を上げる。
pub const THINK_CAND_REVISE_CHEAP_ADV_SQ_NEAR_HUM_KING: u16 = 0xF4B6;

/// 末端局面評価修正: 自分から角をぶつける手を抑制する。
pub const THINK_CAND_REVISE_INHIBIT_BISHOP_EXCHANGE: u16 = 0xF4ED;

/// 末端局面評価修正: 戦力が豊富なとき、手駒の飛車角を温存する手は COM 玉の危険度が高いほど評価を下げる。
pub const THINK_CAND_REVISE_KEEP_ROOK_BISHOP_IN_EMERGENCY: u16 = 0xF50A;

/// 末端局面評価修正: 優勢なときは高い駒を取りながら HUM 玉に迫る手の評価を上げ、駒損を軽視する。
pub const THINK_CAND_REVISE_CAPTURE_NEAR_HUM_KING: u16 = 0xF561;

/// 末端局面評価修正: COM 玉が危険な場合、玉による駒取りは価値なしとする。
pub const THINK_CAND_REVISE_CAPTURE_BY_KING_IN_EMERGENCY: u16 = 0xF58B;

/// 末端局面評価修正: 戦力が豊富なら駒を取りつつ王手する手の評価を上げる。
pub const THINK_CAND_REVISE_CAPTURING_CHECK: u16 = 0xF5A5;

/// 末端局面評価修正: ある程度戦力があるとき、安い駒取りを score_posi に応じて水増しする。
pub const THINK_CAND_REVISE_CHEAP_CAPTURE_PRICE: u16 = 0xF5C0;

/// 末端局面評価修正: 大駒を敵陣以外に打つ手の評価を下げる。
pub const THINK_CAND_REVISE_BAD_ROOK_BISHOP_DROP_2: u16 = 0xF5DF;

/// 末端局面評価修正: 成駒を動かす場合、HUM 玉に近づく手の方を高く評価する。
pub const THINK_CAND_REVISE_PROMOTED_WALK: u16 = 0xF5FF;

/// 末端局面評価修正: 戦力が豊富なら王手の評価を上げる。
pub const THINK_CAND_REVISE_CHECK_WITH_POWER: u16 = 0xF631;

/// 末端局面評価修正: 高い駒を取りながらの王手の評価を上げる。
pub const THINK_CAND_REVISE_GOOD_CAPTURING_CHECK: u16 = 0xF651;

/// 末端局面の評価の修正が完了した。
pub const THINK_CAND_REVISED: u16 = 0xF674;

/// 比較: 候補手は自殺手。最善手が自殺手かどうかは判定済み。
pub const THINK_CAND_IS_SUICIDE: u16 = 0xF680;

/// 比較: 候補手が自殺手でない。最善手が自殺手かどうかは判定済み。
pub const THINK_CAND_IS_NOT_SUICIDE: u16 = 0xF68A;

/// 比較: 候補手は `score_nega` で劣る。`capture_price` は比較済み。
pub const THINK_CAND_CMP_NEGA_WORSE: u16 = 0xF80E;

/// 比較: 候補手は `score_nega` で劣り、`capture_price` で優る。両者の差分は比較済み。
pub const THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_BETTER: u16 = 0xF821;

/// 比較: 候補手は `score_nega` で劣り、`capture_price` が等しい。`power_com` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_1: u16 = 0xF82D;

/// 比較: 候補手は `score_nega` で劣り、`capture_price` が等しい。`capture_price` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_2: u16 = 0xF834;

/// 比較: 候補手は `score_nega` で劣り、`capture_price` が等しい。`score_posi` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_3: u16 = 0xF83C;

/// 比較: 候補手は `score_nega` で劣り、`capture_price` が等しい。
/// `score_posi`, `score_nega` の差分を比較済み。
pub const THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_4: u16 = 0xF84F;

/// 比較: 候補手は `score_nega` で優る。最善手の `score_nega` が極端に大きくないか判定済み。
pub const THINK_CAND_CMP_NEGA_BETTER_1: u16 = 0xF69F;

/// 比較: 候補手は `score_nega` で優る。`capture_price` は比較済み。
pub const THINK_CAND_CMP_NEGA_BETTER_2: u16 = 0xF6A7;

/// 比較: 候補手は `score_nega` で優り、`capture_price` で劣る。両者の差分は比較済み。
pub const THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_WORSE: u16 = 0xF6BA;

/// 比較: 候補手は `score_nega` で優り、`capture_price` が等しい。`power_com` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_1: u16 = 0xF6D2;

/// 比較: 候補手は `score_nega` で優り、`capture_price` が等しい。`capture_price` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_2: u16 = 0xF6D7;

/// 比較: 候補手は `score_nega` で優り、`capture_price` が等しい。`score_posi` 条件を判定済み。
pub const THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_3: u16 = 0xF6DF;

/// 比較: 候補手は `score_nega` で優り、`capture_price` が等しい。
/// `score_posi`, `score_nega` の差分を比較済み。
pub const THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_4: u16 = 0xF6F2;

/// 比較: 候補手は `score_nega` が等しい。`capture_price` は比較済み。
pub const THINK_CAND_CMP_NEGA_EQUAL: u16 = 0xF772;

/// 比較: COM 側の成駒の個数で判定。
pub const THINK_CAND_CMP_COM_PROMO_COUNT: u16 = 0xF77C;

/// 比較: `score_posi` の優劣で判定。
pub const THINK_CAND_CMP_SCORE_POSI: u16 = 0xF786;

/// 比較: `adv_price` の優劣で判定。
pub const THINK_CAND_CMP_ADV_PRICE: u16 = 0xF790;

/// 比較: 候補手は駒打ち。COM 玉に王手がかかっているか判定済み。
pub const THINK_CAND_CMP_DROP_1: u16 = 0xF7DF;

/// 比較: 候補手は合駒を打つ手。より安い駒かどうか判定済み。
pub const THINK_CAND_CMP_DROP_2: u16 = 0xF7E7;

/// 比較: 候補手は盤上の駒を動かす手。`hum_king_threat_around25` の優劣で判定。
pub const THINK_CAND_CMP_WALK_HUM_KING_THREAT_AROUND25: u16 = 0xF7A1;

/// 比較: 候補手は盤上の駒を動かす手。`com_king_safety_around25` の優劣で判定。
pub const THINK_CAND_CMP_WALK_COM_KING_SAFETY_AROUND25: u16 = 0xF7AB;

/// 比較: 候補手は盤上の駒を動かす手。`com_king_threat_around25` の優劣で判定。
pub const THINK_CAND_CMP_WALK_COM_KING_THREAT_AROUND25: u16 = 0xF7B5;

/// 比較: 候補手は盤上の駒を動かす手。COM 側の離れ駒の個数で判定。
pub const THINK_CAND_CMP_WALK_COM_LOOSE_COUNT: u16 = 0xF7BF;

/// 比較: 候補手は COM 玉から遠い駒を動かす手。移動先から HUM 玉への距離で判定。
pub const THINK_CAND_CMP_WALK_DST_TO_HUM_KING: u16 = 0xF7F7;

/// 比較: 候補手は盤上の駒を動かす手。移動元から COM 玉への距離で判定。
pub const THINK_CAND_CMP_WALK_SRC_TO_COM_KING: u16 = 0xF801;

/// 盤上の駒を動かす候補手の処理が完了した。
pub const THINK_CAND_END_WALK: u16 = 0xF0F8;

/// 駒打ちの候補手の処理が完了した。
pub const THINK_CAND_END_DROP: u16 = 0xF25C;

/// 定跡: 処理開始。
pub const THINK_BOOK_START: u16 = 0xE7F8;

/// 定跡: 定跡手を採用するかどうかの判定直後。
pub const THINK_BOOK_JUDGE_MOVE: u16 = 0xE906;

/// 思考ルーチンが終了し、HUM の自殺手と判定された。
pub const THINK_END_HUM_SUICIDE: u16 = 0xDD44;

/// 思考ルーチンが終了し、HUM の勝ちと判定された。
pub const THINK_END_HUM_WIN: u16 = 0xDD47;

/// 思考ルーチンが終了し、COM の勝ちと判定された。
pub const THINK_END_COM_WIN: u16 = 0xDFD6;

/// 思考ルーチンが終了し、通常の指し手を返した。
pub const THINK_END_MOVE: u16 = 0xDFD3;
