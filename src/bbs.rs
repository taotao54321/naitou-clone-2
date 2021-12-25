//! 各種 bitboard テーブル、およびそれらを用いた計算。**プログラム起動直後に必ず `init()` で初期化すること**。
//!
//! ユニットテスト時は `init()` を呼ばなくても自動で初期化される。
//! 統合テストで本モジュールを用いる際は冒頭で `init()` を呼ぶ必要がある。

// 飛び利きの計算についてはやねうら王のコード、および以下の資料を参照:
//
// * [Qugiyの飛び利きのコード、完全解説 | やねうら王 公式サイト](https://yaneuraou.yaneu.com/2021/12/03/qugiys-jumpy-effect-code-complete-guide/)

// とりあえず全テーブルを個別に OnceCell に入れておく。
// これにより、初期化中でも既に初期化したテーブルに簡単にアクセスできるが、
// 代償として OnceCell からの取り出し回数は増える。
//
// そこで、プログラム起動直後に init() で全テーブルを初期化し、
// テーブルへのアクセスには OnceCell::get_unchecked() を使うことにする。
// ただし、デバッグビルドおよびテストでは OnceCell::get_or_init() を使い、初期化を保証する。
//
// init() 内では正しい順序で初期化を行う必要がある。間違えるとリリースビルドで壊れる。

use once_cell::sync::OnceCell;

use crate::bitboard::{Bitboard, Bitboard256};
use crate::myarray::*;
use crate::shogi::*;

/// bitboard テーブルたちを初期化する。**プログラム起動直後に必ずこれを呼ぶこと**。
pub fn init() {
    // これらは他のテーブル初期化時によく使われるので最初に初期化する。
    // これら自身は何にも依存せず初期化できる。
    BB_COL.get_or_init(init_col);
    BB_ROW.get_or_init(init_row);
    BB_SQUARE.get_or_init(init_square);

    // 香の利きを求めるのに forward_rows() が必要。
    BB_FORWARD_ROWS.get_or_init(init_forward_rows);
    BB_PROMOTION_ZONE.get_or_init(init_promotion_zone);

    // 飛車、角の利き計算用のテーブル。
    BB_QUGIY_ROOK_MASK.get_or_init(init_qugiy_rook_mask);
    BB_QUGIY_BISHOP_MASK.get_or_init(init_qugiy_bishop_mask);

    // 飛車の利きを求めるのに lance_effect() が必要。
    BB_PAWN_EFFECT.get_or_init(init_pawn_effect);
    BB_LANCE_STEP_EFFECT.get_or_init(init_lance_step_effect);

    // これ以降、lance_effect(), rook_effect(), bishop_effect() が使える。
    // 他の駒の利きテーブルはこれらを使って初期化される。
    BB_KING_EFFECT.get_or_init(init_king_effect);
    BB_ROOK_STEP_SFFECT.get_or_init(init_rook_step_effect);
    BB_BISHOP_STEP_EFFECT.get_or_init(init_bishop_step_effect);
    BB_GOLD_EFFECT.get_or_init(init_gold_effect);
    BB_SILVER_EFFECT.get_or_init(init_silver_effect);
    BB_KNIGHT_EFFECT.get_or_init(init_knight_effect);

    BB_AROUND25.get_or_init(init_around25);
}

#[cfg(any(debug_assertions, test))]
fn once_cell_get<T, F>(cell: &OnceCell<T>, f: F) -> &T
where
    F: FnOnce() -> T,
{
    cell.get_or_init(f)
}

#[cfg(not(any(debug_assertions, test)))]
fn once_cell_get<T, F>(cell: &OnceCell<T>, _f: F) -> &T
where
    F: FnOnce() -> T,
{
    unsafe { cell.get_unchecked() }
}

type BbCol = MyArray1<Bitboard, Col, 9>;
type BbRow = MyArray1<Bitboard, Row, 9>;
type BbSquare = MyArray1<Bitboard, Square, 81>;
type BbForwardRows = MyArray2<Bitboard, Side, Row, 2, 9>;
type BbPromotionZone = MyArray1<Bitboard, Side, 2>;
type BbQugiyRookMask = MyArray2<Bitboard, Square, usize, 81, 2>;
type BbQugiyBishopMask = MyArray2<Bitboard256, Square, usize, 81, 2>;
type BbPawnEffect = MyArray2<Bitboard, Square, Side, 81, 2>;
type BbLanceStepEffect = MyArray2<Bitboard, Square, Side, 81, 2>;
type BbKingEffect = MyArray1<Bitboard, Square, 81>;
type BbRookStepEffect = MyArray1<Bitboard, Square, 81>;
type BbBishopStepEffect = MyArray1<Bitboard, Square, 81>;
type BbGoldEffect = MyArray2<Bitboard, Square, Side, 81, 2>;
type BbSilverEffect = MyArray2<Bitboard, Square, Side, 81, 2>;
type BbKnightEffect = MyArray2<Bitboard, Square, Side, 81, 2>;
type BbAround25 = MyArray1<Bitboard, Square, 81>;

static BB_COL: OnceCell<BbCol> = OnceCell::new();
static BB_ROW: OnceCell<BbRow> = OnceCell::new();
static BB_SQUARE: OnceCell<BbSquare> = OnceCell::new();
static BB_FORWARD_ROWS: OnceCell<BbForwardRows> = OnceCell::new();
static BB_PROMOTION_ZONE: OnceCell<BbPromotionZone> = OnceCell::new();
static BB_QUGIY_ROOK_MASK: OnceCell<BbQugiyRookMask> = OnceCell::new();
static BB_QUGIY_BISHOP_MASK: OnceCell<BbQugiyBishopMask> = OnceCell::new();
static BB_PAWN_EFFECT: OnceCell<BbPawnEffect> = OnceCell::new();
static BB_LANCE_STEP_EFFECT: OnceCell<BbLanceStepEffect> = OnceCell::new();
static BB_KING_EFFECT: OnceCell<BbKingEffect> = OnceCell::new();
static BB_ROOK_STEP_SFFECT: OnceCell<BbRookStepEffect> = OnceCell::new();
static BB_BISHOP_STEP_EFFECT: OnceCell<BbBishopStepEffect> = OnceCell::new();
static BB_GOLD_EFFECT: OnceCell<BbGoldEffect> = OnceCell::new();
static BB_SILVER_EFFECT: OnceCell<BbSilverEffect> = OnceCell::new();
static BB_KNIGHT_EFFECT: OnceCell<BbKnightEffect> = OnceCell::new();
static BB_AROUND25: OnceCell<BbAround25> = OnceCell::new();

/// 与えられた筋を表す bitboard を返す。
pub fn col(col: Col) -> Bitboard {
    let bb = once_cell_get(&BB_COL, init_col);
    bb[col]
}

/// 与えられた段を表す bitboard を返す。
pub fn row(row: Row) -> Bitboard {
    let bb = once_cell_get(&BB_ROW, init_row);
    bb[row]
}

/// 与えられたマスを表す bitboard を返す。
pub fn square(sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_SQUARE, init_square);
    bb[sq]
}

/// `side` から見て `row` より敵陣側の段たちを表す bitboard を返す。
pub fn forward_rows(side: Side, row: Row) -> Bitboard {
    let bb = once_cell_get(&BB_FORWARD_ROWS, init_forward_rows);
    bb[side][row]
}

/// `side` から見た敵陣を表す bitboard を返す。
pub fn promotion_zone(side: Side) -> Bitboard {
    let bb = once_cell_get(&BB_PROMOTION_ZONE, init_promotion_zone);
    bb[side]
}

/// `side` 側が `sq` に置いた歩の利きを返す。
pub fn pawn_effect(side: Side, sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_PAWN_EFFECT, init_pawn_effect);
    bb[sq][side]
}

/// `side` 側が `sq` に置いた香の step effect を返す。
pub fn lance_step_effect(side: Side, sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_LANCE_STEP_EFFECT, init_lance_step_effect);
    bb[sq][side]
}

/// 盤面 `occ` において `side` 側が `sq` に置いた香の利きを返す。
pub fn lance_effect(side: Side, sq: Square, occ: Bitboard) -> Bitboard {
    if side == HUM {
        lance_effect_hum(sq, occ)
    } else {
        lance_effect_com(sq, occ)
    }
}

/// 盤面 `occ` において HUM が `sq` に置いた香の利きを返す。
fn lance_effect_hum(sq: Square, occ: Bitboard) -> Bitboard {
    fn effect(step_eff: u64, occ: u64) -> u64 {
        // step effect 上にある駒のみが利きに影響する。
        let mut mask = step_eff & occ;

        // HUM の香の利きは盤上の駒より上のマスには届かない。
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask >>= 1;

        step_eff & !mask
    }

    let step_eff = lance_step_effect(HUM, sq);

    if Bitboard::square_is_part0(sq) {
        let step_eff_lo = step_eff.part0();
        let occ_lo = occ.part0();
        Bitboard::from_parts(effect(step_eff_lo, occ_lo), 0)
    } else {
        let step_eff_hi = step_eff.part1();
        let occ_hi = occ.part1();
        Bitboard::from_parts(0, effect(step_eff_hi, occ_hi))
    }
}

/// 盤面 `occ` において COM が `sq` に置いた香の利きを返す。
fn lance_effect_com(sq: Square, occ: Bitboard) -> Bitboard {
    fn effect(step_eff: u64, occ: u64) -> u64 {
        // step effect 上にある駒のみが利きに影響する。
        let mask = step_eff & occ;

        // (x ^ (x-1)) は最下位の 1 以下のビットを 1 に、他を 0 にする。
        // 1 のビットがなければ全ビットが 1 になる。
        (mask ^ (mask.wrapping_sub(1))) & step_eff
    }

    let step_eff = lance_step_effect(COM, sq);

    if Bitboard::square_is_part0(sq) {
        let step_eff_lo = step_eff.part0();
        let occ_lo = occ.part0();
        Bitboard::from_parts(effect(step_eff_lo, occ_lo), 0)
    } else {
        let step_eff_hi = step_eff.part1();
        let occ_hi = occ.part1();
        Bitboard::from_parts(0, effect(step_eff_hi, occ_hi))
    }
}

/// 盤面 `occ` において `sq` に置いた飛車の利きを返す。
pub fn rook_effect(sq: Square, occ: Bitboard) -> Bitboard {
    rook_col_effect(sq, occ) | rook_row_effect(sq, occ)
}

/// 盤面 `occ` において `sq` に置いた飛車の縦利きを返す。
fn rook_col_effect(sq: Square, occ: Bitboard) -> Bitboard {
    // HUM の香の利きと COM の香の利きを合成すればよい。
    // ベタ書きするのに比べ Bitboard::square_is_part0() の判定が 1 回増えるが、
    // これくらいは最適化されてほしい。

    lance_effect_hum(sq, occ) | lance_effect_com(sq, occ)
}

/// 盤面 `occ` において `sq` に置いた飛車の横利きを返す。
fn rook_row_effect(sq: Square, occ: Bitboard) -> Bitboard {
    let qrm_lo = qugiy_rook_mask(sq, 0);
    let qrm_hi = qugiy_rook_mask(sq, 1);

    // 右方向の利きについてはバイト反転されているので、occ も同様にする必要がある。
    let occ_rev = occ.byte_reverse();

    // occ, occ_rev を unpack する。
    let (occ_unp_lo, occ_unp_hi) = Bitboard::unpack_pair(occ, occ_rev);

    // step effect 上にある駒のみが利きに影響する。
    let mask_lo = qrm_lo & occ_unp_lo;
    let mask_hi = qrm_hi & occ_unp_hi;

    // デクリメントすることで利きを求める。
    // 香の利きを求める際に (x ^ (x-1)) とするのと同様。
    let (mask_lo_dec, mask_hi_dec) = Bitboard::decrement_unpacked_pair(mask_lo, mask_hi);
    let eff_unp_lo = (mask_lo ^ mask_lo_dec) & qrm_lo;
    let eff_unp_hi = (mask_hi ^ mask_hi_dec) & qrm_hi;

    // 求まった利きは unpack されているので、再度 unpack して元に戻す。
    let (eff_left, eff_right_rev) = Bitboard::unpack_pair(eff_unp_lo, eff_unp_hi);

    // 右方向の利きはバイト反転しているので、元に戻す。
    let eff_right = eff_right_rev.byte_reverse();

    eff_left | eff_right
}

/// 盤面 `occ` において `sq` に置いた角の利きを返す。
pub fn bishop_effect(sq: Square, occ: Bitboard) -> Bitboard {
    let qbm_lo = qugiy_bishop_mask(sq, 0);
    let qbm_hi = qugiy_bishop_mask(sq, 1);

    // 4 方向を一度に処理するため、occ を 2 枚並べた Bitboard256 を用意する。
    // 右上/右下方向については利きがバイト反転されているので、同様に occ を反転する。
    let occ2 = Bitboard256::broadcast_bitboard(occ);
    let occ2_rev = Bitboard256::broadcast_bitboard(occ.byte_reverse());

    // occ2, occ2_rev を unpack する。
    let (occ2_unp_lo, occ2_unp_hi) = Bitboard256::unpack_pair(occ2, occ2_rev);

    // step effect 上にある駒のみが利きに影響する。
    let mask_lo = qbm_lo & occ2_unp_lo;
    let mask_hi = qbm_hi & occ2_unp_hi;

    // デクリメントすることで利きを求める。
    // 香の利きを求める際に (x ^ (x-1)) とするのと同様。
    let (mask_lo_dec, mask_hi_dec) = Bitboard256::decrement_unpacked_pair(mask_lo, mask_hi);
    let eff_unp_lo = (mask_lo ^ mask_lo_dec) & qbm_lo;
    let eff_unp_hi = (mask_hi ^ mask_hi_dec) & qbm_hi;

    // 求まった利きは unpack されているので、再度 unpack して元に戻す。
    let (eff_left, eff_right_rev) = Bitboard256::unpack_pair(eff_unp_lo, eff_unp_hi);

    // 右上/右下方向の利きはバイト反転しているので、元に戻す。
    let eff_right = eff_right_rev.byte_reverse();

    // 全ての利きを重ね合わせる。
    (eff_left | eff_right).merge()
}

/// `sq` に置いた玉の利きを返す。
pub fn king_effect(sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_KING_EFFECT, init_king_effect);
    bb[sq]
}

/// `sq` に置いた飛車の step effect を返す。
pub fn rook_step_effect(sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_ROOK_STEP_SFFECT, init_rook_step_effect);
    bb[sq]
}

/// `sq` に置いた角の step effect を返す。
pub fn bishop_step_effect(sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_BISHOP_STEP_EFFECT, init_bishop_step_effect);
    bb[sq]
}

/// `side` 側が `sq` に置いた金の利きを返す。
pub fn gold_effect(side: Side, sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_GOLD_EFFECT, init_gold_effect);
    bb[sq][side]
}

/// `side` 側が `sq` に置いた銀の利きを返す。
pub fn silver_effect(side: Side, sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_SILVER_EFFECT, init_silver_effect);
    bb[sq][side]
}

/// `side` 側が `sq` に置いた桂の利きを返す。
pub fn knight_effect(side: Side, sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_KNIGHT_EFFECT, init_knight_effect);
    bb[sq][side]
}

/// 盤面 `occ` において `sq` に置いた龍の利きを返す。
pub fn dragon_effect(sq: Square, occ: Bitboard) -> Bitboard {
    rook_effect(sq, occ) | king_effect(sq)
}

/// 盤面 `occ` において `sq` に置いた馬の利きを返す。
pub fn horse_effect(sq: Square, occ: Bitboard) -> Bitboard {
    bishop_effect(sq, occ) | king_effect(sq)
}

/// `sq` を中心とする縦横十字の長さ 1 の利きを返す。
pub fn axis_cross_effect(sq: Square) -> Bitboard {
    rook_step_effect(sq) & king_effect(sq)
}

/// `sq` を中心とする斜め十字の長さ 1 の利きを返す。
pub fn diagonal_cross_effect(sq: Square) -> Bitboard {
    bishop_step_effect(sq) & king_effect(sq)
}

/// 盤面 `occ` において `sq` に置いたクイーンの利きを返す。
pub fn queen_effect(sq: Square, occ: Bitboard) -> Bitboard {
    bishop_effect(sq, occ) | rook_effect(sq, occ)
}

/// 盤面 `occ` において駒 `pc` を `sq` に置いたときの利きを返す。
/// `pc` は実際の駒でなければならない。
pub fn effect(pc: Piece, sq: Square, occ: Bitboard) -> Bitboard {
    debug_assert!(pc.is_piece());

    match pc {
        H_PAWN => pawn_effect(HUM, sq),
        H_LANCE => lance_effect(HUM, sq, occ),
        H_KNIGHT => knight_effect(HUM, sq),
        H_SILVER => silver_effect(HUM, sq),
        H_GOLD | H_PRO_PAWN | H_PRO_LANCE | H_PRO_KNIGHT | H_PRO_SILVER => gold_effect(HUM, sq),

        C_PAWN => pawn_effect(COM, sq),
        C_LANCE => lance_effect(COM, sq, occ),
        C_KNIGHT => knight_effect(COM, sq),
        C_SILVER => silver_effect(COM, sq),
        C_GOLD | C_PRO_PAWN | C_PRO_LANCE | C_PRO_KNIGHT | C_PRO_SILVER => gold_effect(COM, sq),

        H_BISHOP | C_BISHOP => bishop_effect(sq, occ),
        H_ROOK | C_ROOK => rook_effect(sq, occ),
        H_HORSE | C_HORSE => horse_effect(sq, occ),
        H_DRAGON | C_DRAGON => dragon_effect(sq, occ),
        H_KING | C_KING => king_effect(sq),

        _ => unreachable!(),
    }
}

/// 駒 `pc` を `sq` に置いたときの近接利きを返す。
/// `pc` は実際の駒でなければならない。
pub fn effect_melee(pc: Piece, sq: Square) -> Bitboard {
    debug_assert!(pc.is_piece());

    match pc {
        H_PAWN => pawn_effect(HUM, sq),
        H_KNIGHT => knight_effect(HUM, sq),
        H_SILVER => silver_effect(HUM, sq),
        H_GOLD | H_PRO_PAWN | H_PRO_LANCE | H_PRO_KNIGHT | H_PRO_SILVER => gold_effect(HUM, sq),

        C_PAWN => pawn_effect(COM, sq),
        C_KNIGHT => knight_effect(COM, sq),
        C_SILVER => silver_effect(COM, sq),
        C_GOLD | C_PRO_PAWN | C_PRO_LANCE | C_PRO_KNIGHT | C_PRO_SILVER => gold_effect(COM, sq),

        H_HORSE | C_HORSE => axis_cross_effect(sq),
        H_DRAGON | C_DRAGON => diagonal_cross_effect(sq),
        H_KING | C_KING => king_effect(sq),

        H_LANCE | C_LANCE | H_BISHOP | C_BISHOP | H_ROOK | C_ROOK => Bitboard::zero(),

        _ => unreachable!(),
    }
}

/// `side` 側の歩の bitboard に対し、それらの歩の利きの bitboard を返す。
///
/// HUM の場合、敵陣一段目の歩があると隣の筋の九段目に利きが発生するので注意。
pub fn pawn_bb_effect(side: Side, bb: Bitboard) -> Bitboard {
    if side == HUM {
        bb.logical_shift_right_parts::<1>()
    } else {
        bb.logical_shift_left_parts::<1>()
    }
}

/// `side` 側の歩の配置が `pawns` であるときに `side` 側が歩を打てる位置の bitboard を返す。
pub fn pawn_drop_mask(side: Side, pawns: Bitboard) -> Bitboard {
    // ref: [WCSC31 Qugiy アピール文書](https://www.apply.computer-shogi.org/wcsc31/appeal/Qugiy/appeal.pdf)

    // t1 = left - pawns を図示すると以下のようになる(0 のビットは '.' で表す。pawns の値は一例):
    //
    // .........   .........   .........
    // .........   ......1..   ......1..
    // .........   .1.......   .1....1..
    // .........   ........1   .1....1.1
    // ......... - 1........ = 11....1.1
    // .........   .........   11....1.1
    // .........   ..1......   111...1.1
    // .........   .........   111...1.1
    // 111111111   ....1....   ...1.1.1. <-- 9 段目のみに注目。他の段はどうでも良い
    //
    // この演算で t1 の 9 段目が歩のない筋を表すようになる。これを筋全体に広げればよい。
    // そこで、t2 = (t1 & left) >> 8 とする。これは 1 段目が歩のない筋を表し、他の段は全て 0。
    // そして演算 left - t2 を行う。図示すると以下のようになる:
    //
    // .........   ...1.1.1.   ...1.1.1.
    // .........   .........   ...1.1.1.
    // .........   .........   ...1.1.1.
    // .........   .........   ...1.1.1.
    // ......... - ......... = ...1.1.1.
    // .........   .........   ...1.1.1.
    // .........   .........   ...1.1.1.
    // .........   .........   ...1.1.1.
    // 111111111   .........   111.1.1.1
    //
    // この結果と left の XOR をとれば、歩のない筋全体が 1 になった bitboard が得られる。
    //
    // ただし実際は HUM なら 1 段目、COM なら 9 段目は 0 になってほしいので少し修正する。
    // (やねうら王からのパクリ)

    let left = Bitboard::from_parts(
        (1 << 8) | (1 << 17) | (1 << 26) | (1 << 35) | (1 << 44) | (1 << 53) | (1 << 62),
        (1 << 8) | (1 << 17),
    );

    let t1 = left.sub_parts(pawns);

    if side == HUM {
        // 1 段目ではなく 2 段目に移動させる。後は上記と同様。
        let t2 = (t1 & left).logical_shift_right_parts::<7>();
        left ^ left.sub_parts(t2)
    } else {
        // 1 段目に移動させるが、最後の XOR を ANDNOT に変える。
        let t2 = (t1 & left).logical_shift_right_parts::<8>();
        left.andnot(left.sub_parts(t2))
    }
}

/// 飛車の横利き計算用のマスクを返す。
fn qugiy_rook_mask(sq: Square, idx: usize) -> Bitboard {
    let bb = once_cell_get(&BB_QUGIY_ROOK_MASK, init_qugiy_rook_mask);
    bb[sq][idx]
}

/// 角の利き計算用のマスクを返す。
fn qugiy_bishop_mask(sq: Square, idx: usize) -> Bitboard256 {
    let bb = once_cell_get(&BB_QUGIY_BISHOP_MASK, init_qugiy_bishop_mask);
    bb[sq][idx]
}

/// `sq` からチェス盤距離 2 以内のマスが 1 である bitboard を返す。
/// 盤面外にはみ出さない場合、横幅 5, 縦幅 5 の計 25 マスとなる。
pub fn around25(sq: Square) -> Bitboard {
    let bb = once_cell_get(&BB_AROUND25, init_around25);
    bb[sq]
}

#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
fn init_col() -> BbCol {
    [
        Bitboard::from_parts(0x1FF << (9 * 0), 0),
        Bitboard::from_parts(0x1FF << (9 * 1), 0),
        Bitboard::from_parts(0x1FF << (9 * 2), 0),
        Bitboard::from_parts(0x1FF << (9 * 3), 0),
        Bitboard::from_parts(0x1FF << (9 * 4), 0),
        Bitboard::from_parts(0x1FF << (9 * 5), 0),
        Bitboard::from_parts(0x1FF << (9 * 6), 0),
        Bitboard::from_parts(0, 0x1FF << (9 * 0)),
        Bitboard::from_parts(0, 0x1FF << (9 * 1)),
    ]
    .into()
}

#[allow(clippy::identity_op)]
fn init_row() -> BbRow {
    [
        Bitboard::from_parts(0x40201008040201 << 0, 0x201 << 0),
        Bitboard::from_parts(0x40201008040201 << 1, 0x201 << 1),
        Bitboard::from_parts(0x40201008040201 << 2, 0x201 << 2),
        Bitboard::from_parts(0x40201008040201 << 3, 0x201 << 3),
        Bitboard::from_parts(0x40201008040201 << 4, 0x201 << 4),
        Bitboard::from_parts(0x40201008040201 << 5, 0x201 << 5),
        Bitboard::from_parts(0x40201008040201 << 6, 0x201 << 6),
        Bitboard::from_parts(0x40201008040201 << 7, 0x201 << 7),
        Bitboard::from_parts(0x40201008040201 << 8, 0x201 << 8),
    ]
    .into()
}

fn init_square() -> BbSquare {
    let mut bb_square = BbSquare::default();

    for sq in Square::iter() {
        let col = sq.col();
        let row = sq.row();

        let (lo, hi) = if col <= COL_7 {
            (1 << (9 * u32::from(col) + u32::from(row)), 0)
        } else {
            (
                0,
                1 << (9 * (u32::from(col) - u32::from(COL_8)) + u32::from(row)),
            )
        };

        bb_square[sq] = Bitboard::from_parts(lo, hi);
    }

    bb_square
}

fn init_forward_rows() -> BbForwardRows {
    fn f(row: Row) -> Bitboard {
        self::row(row)
    }

    [
        [
            Bitboard::zero(),
            f(ROW_1),
            f(ROW_1) | f(ROW_2),
            f(ROW_1) | f(ROW_2) | f(ROW_3),
            f(ROW_1) | f(ROW_2) | f(ROW_3) | f(ROW_4),
            f(ROW_1) | f(ROW_2) | f(ROW_3) | f(ROW_4) | f(ROW_5),
            f(ROW_1) | f(ROW_2) | f(ROW_3) | f(ROW_4) | f(ROW_5) | f(ROW_6),
            f(ROW_1) | f(ROW_2) | f(ROW_3) | f(ROW_4) | f(ROW_5) | f(ROW_6) | f(ROW_7),
            f(ROW_1) | f(ROW_2) | f(ROW_3) | f(ROW_4) | f(ROW_5) | f(ROW_6) | f(ROW_7) | f(ROW_8),
        ]
        .into(),
        [
            f(ROW_9) | f(ROW_8) | f(ROW_7) | f(ROW_6) | f(ROW_5) | f(ROW_4) | f(ROW_3) | f(ROW_2),
            f(ROW_9) | f(ROW_8) | f(ROW_7) | f(ROW_6) | f(ROW_5) | f(ROW_4) | f(ROW_3),
            f(ROW_9) | f(ROW_8) | f(ROW_7) | f(ROW_6) | f(ROW_5) | f(ROW_4),
            f(ROW_9) | f(ROW_8) | f(ROW_7) | f(ROW_6) | f(ROW_5),
            f(ROW_9) | f(ROW_8) | f(ROW_7) | f(ROW_6),
            f(ROW_9) | f(ROW_8) | f(ROW_7),
            f(ROW_9) | f(ROW_8),
            f(ROW_9),
            Bitboard::zero(),
        ]
        .into(),
    ]
    .into()
}

fn init_promotion_zone() -> BbPromotionZone {
    fn f(row: Row) -> Bitboard {
        self::row(row)
    }

    [
        f(ROW_1) | f(ROW_2) | f(ROW_3),
        f(ROW_9) | f(ROW_8) | f(ROW_7),
    ]
    .into()
}

fn init_qugiy_rook_mask() -> BbQugiyRookMask {
    let mut bb_qugiy_rook_mask = BbQugiyRookMask::default();

    for sq in Square::iter() {
        let col = sq.col();
        let row = sq.row();

        let mut left = Bitboard::zero();
        let mut right = Bitboard::zero();

        // 左方向の利き
        for col_dst in Col::iter_range(col + 1, COL_9) {
            let dst = Square::from_col_row(col_dst, row);
            left |= Bitboard::from(dst);
        }

        // 右方向の利き
        for col_dst in Col::iter_range(COL_1, col - 1) {
            let dst = Square::from_col_row(col_dst, row);
            right |= Bitboard::from(dst);
        }

        // 右方向の利きについてはバイト反転する。
        // これにより、利きがビット位置について昇順になる。
        let right_rev = right.byte_reverse();

        // unpack した形でテーブルに格納する。
        let (qrm_lo, qrm_hi) = Bitboard::unpack_pair(left, right_rev);

        bb_qugiy_rook_mask[sq][0] = qrm_lo;
        bb_qugiy_rook_mask[sq][1] = qrm_hi;
    }

    bb_qugiy_rook_mask
}

fn init_qugiy_bishop_mask() -> BbQugiyBishopMask {
    const DIRS: [i32; 4] = [
        SquareWithWall::DIR_LU,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_RU,
        SquareWithWall::DIR_RD,
    ];

    // start から dir の方向のマスを列挙する。始点は含まない。
    fn squares_of_dir(start: Square, dir: i32) -> impl Iterator<Item = Square> {
        let mut sq_ww = SquareWithWall::from(start) + dir;
        std::iter::from_fn(move || {
            sq_ww.is_on_board().then(|| {
                let res = Square::from(sq_ww);
                sq_ww += dir;
                res
            })
        })
    }

    let mut bb_qugiy_bishop_mask = BbQugiyBishopMask::default();

    for sq in Square::iter() {
        let mut step_effs = [Bitboard::zero(); 4];

        // 各方向について盤面外に出るまで進み、step effect を求める。
        for (i, dir) in DIRS.into_iter().enumerate() {
            for dst in squares_of_dir(sq, dir) {
                step_effs[i] |= Bitboard::from(dst);
            }
        }

        // 右上/右下方向についてはバイト反転する。
        // これにより、利きがビット位置について昇順になる。
        for eff in &mut step_effs[2..4] {
            *eff = eff.byte_reverse();
        }

        // unpack した形でテーブルに格納する。
        for i in 0..2 {
            bb_qugiy_bishop_mask[sq][i as usize] = Bitboard256::from_bitboards(
                Bitboard::from_parts(step_effs[0].part(i), step_effs[2].part(i)),
                Bitboard::from_parts(step_effs[1].part(i), step_effs[3].part(i)),
            );
        }
    }

    bb_qugiy_bishop_mask
}

fn init_pawn_effect() -> BbPawnEffect {
    let mut bb_pawn_effect = BbPawnEffect::default();

    for sq in Square::iter() {
        for side in Side::iter() {
            let row = sq.row() + if side == HUM { -1 } else { 1 };
            let bb = if row.is_on_board() {
                let dst = Square::from_col_row(sq.col(), row);
                self::square(dst)
            } else {
                Bitboard::zero()
            };
            bb_pawn_effect[sq][side] = bb;
        }
    }

    bb_pawn_effect
}

fn init_lance_step_effect() -> BbLanceStepEffect {
    let mut bb_lance_step_effect = BbLanceStepEffect::default();

    for sq in Square::iter() {
        for side in Side::iter() {
            let col = sq.col();
            let row = sq.row();
            let bb = self::col(col) & forward_rows(side, row);
            bb_lance_step_effect[sq][side] = bb;
        }
    }

    bb_lance_step_effect
}

fn init_king_effect() -> BbKingEffect {
    let mut bb_king_effect = BbKingEffect::default();

    for sq in Square::iter() {
        // 玉の利きは、駒が敷き詰められているときの飛車と角の利きを合成したもの。
        bb_king_effect[sq] = rook_effect(sq, Bitboard::all()) | bishop_effect(sq, Bitboard::all());
    }

    bb_king_effect
}

fn init_rook_step_effect() -> BbRookStepEffect {
    let mut bb_rook_step_effect = BbRookStepEffect::default();

    for sq in Square::iter() {
        // 盤上に駒がないときの飛車の利きを求めればよい。
        bb_rook_step_effect[sq] = rook_effect(sq, Bitboard::zero());
    }

    bb_rook_step_effect
}

fn init_bishop_step_effect() -> BbBishopStepEffect {
    let mut bb_bishop_step_effect = BbBishopStepEffect::default();

    for sq in Square::iter() {
        // 盤上に駒がないときの角の利きを求めればよい。
        bb_bishop_step_effect[sq] = bishop_effect(sq, Bitboard::zero());
    }

    bb_bishop_step_effect
}

fn init_gold_effect() -> BbGoldEffect {
    let mut bb_gold_effect = BbGoldEffect::default();

    for sq in Square::iter() {
        for side in Side::iter() {
            // 駒が敷き詰められているとして、飛車の利きと角の前方利きを合成する。
            // 角の後方利きをマスクするため、`sq` の敵側の歩の利きの段の bitboard を用いる。

            // なるべく依存を減らすため、lance_effect() を用いて敵側の歩の利きを求める。
            let eff_enemy_pawn = lance_effect(side.inv(), sq, Bitboard::all());

            let mask = if eff_enemy_pawn.is_zero() {
                Bitboard::zero()
            } else {
                let r = eff_enemy_pawn.get_least_square().row();
                self::row(r)
            };

            let eff_rook = rook_effect(sq, Bitboard::all());
            let eff_bishop_fwd = bishop_effect(sq, Bitboard::all()) & !mask;

            bb_gold_effect[sq][side] = eff_rook | eff_bishop_fwd;
        }
    }

    bb_gold_effect
}

fn init_silver_effect() -> BbSilverEffect {
    let mut bb_silver_effect = BbSilverEffect::default();

    for sq in Square::iter() {
        for side in Side::iter() {
            // 長さ 1 の角の利きと長さ 1 の香の利きを合成すればよい。
            let eff_bishop = bishop_effect(sq, Bitboard::all());
            let eff_lance = lance_effect(side, sq, Bitboard::all());

            bb_silver_effect[sq][side] = eff_bishop | eff_lance;
        }
    }

    bb_silver_effect
}

fn init_knight_effect() -> BbKnightEffect {
    let mut bb_knight_effect = BbKnightEffect::default();

    for sq in Square::iter() {
        for side in Side::iter() {
            // 歩の利きの地点からの長さ 1 の角の前方利きを求めればよい。

            let mut eff = Bitboard::zero();

            // なるべく依存を減らすため、lance_effect() を用いて歩の利きを求める。
            let eff_pawn = lance_effect(side, sq, Bitboard::all());

            if !eff_pawn.is_zero() {
                let sq2 = eff_pawn.get_least_square();

                // 前方利きを求めるため、さらに 1 つ先の段を求める。
                let eff2_pawn = lance_effect(side, sq2, Bitboard::all());

                if !eff2_pawn.is_zero() {
                    let r = eff2_pawn.get_least_square().row();
                    eff = bishop_effect(sq2, Bitboard::all()) & self::row(r);
                }
            }

            bb_knight_effect[sq][side] = eff;
        }
    }

    bb_knight_effect
}

fn init_around25() -> BbAround25 {
    let mut bb_around25 = BbAround25::default();

    for sq in Square::iter() {
        bb_around25[sq] = Bitboard::zero();

        let cols = Col::iter_range(sq.col() - 2, sq.col() + 2).filter(|&col| col.is_on_board());
        for col in cols {
            let rows = Row::iter_range(sq.row() - 2, sq.row() + 2).filter(|&row| row.is_on_board());
            for row in rows {
                bb_around25[sq] |= Bitboard::from(Square::from_col_row(col, row));
            }
        }
    }

    bb_around25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne};

    fn bb_from_sqs(sqs: impl IntoIterator<Item = Square>) -> Bitboard {
        sqs.into_iter()
            .map(Bitboard::from)
            .reduce(std::ops::BitOr::bitor)
            .expect("sqs must be nonempty")
    }

    #[test]
    fn test_pawn_effect() {
        assert_eq!(pawn_effect(HUM, SQ_79), Bitboard::from(SQ_78));

        assert_eq!(pawn_effect(COM, SQ_81), Bitboard::from(SQ_82));
    }

    #[test]
    fn test_lance_step_effect() {
        assert_eq!(
            lance_step_effect(HUM, SQ_89),
            bb_from_sqs([SQ_88, SQ_87, SQ_86, SQ_85, SQ_84, SQ_83, SQ_82, SQ_81])
        );

        assert_eq!(
            lance_step_effect(HUM, SQ_45),
            bb_from_sqs([SQ_44, SQ_43, SQ_42, SQ_41])
        );

        assert_eq!(
            lance_step_effect(COM, SQ_71),
            bb_from_sqs([SQ_72, SQ_73, SQ_74, SQ_75, SQ_76, SQ_77, SQ_78, SQ_79])
        );

        assert_eq!(lance_step_effect(COM, SQ_18), bb_from_sqs([SQ_19]));
    }

    #[test]
    fn test_lance_effect() {
        {
            let occ = bb_from_sqs([SQ_77, SQ_74, SQ_71]);
            assert_eq!(lance_effect(HUM, SQ_76, occ), bb_from_sqs([SQ_75, SQ_74]));
            assert_eq!(lance_effect(COM, SQ_75, occ), bb_from_sqs([SQ_76, SQ_77]));
        }
        {
            let occ = bb_from_sqs([SQ_82, SQ_88]);
            assert_eq!(
                lance_effect(HUM, SQ_86, occ),
                bb_from_sqs([SQ_85, SQ_84, SQ_83, SQ_82])
            );
            assert_eq!(lance_effect(COM, SQ_81, occ), bb_from_sqs([SQ_82]));
        }
    }

    #[test]
    fn test_rook_effect() {
        let occ = bb_from_sqs([SQ_25, SQ_61, SQ_85, SQ_67]);
        assert_eq!(
            rook_effect(SQ_65, occ),
            bb_from_sqs([
                SQ_55, SQ_45, SQ_35, SQ_25, SQ_64, SQ_63, SQ_62, SQ_61, SQ_66, SQ_67, SQ_75, SQ_85
            ])
        );
    }

    #[test]
    fn test_bishop_effect() {
        let occ = bb_from_sqs([SQ_23, SQ_81, SQ_67, SQ_36]);
        assert_eq!(
            bishop_effect(SQ_45, occ),
            bb_from_sqs([SQ_34, SQ_23, SQ_36, SQ_54, SQ_63, SQ_72, SQ_81, SQ_56, SQ_67])
        );
    }

    #[test]
    fn test_king_effect() {
        assert_eq!(
            king_effect(SQ_85),
            bb_from_sqs([SQ_74, SQ_75, SQ_76, SQ_84, SQ_86, SQ_94, SQ_95, SQ_96])
        );

        assert_eq!(
            king_effect(SQ_79),
            bb_from_sqs([SQ_68, SQ_69, SQ_78, SQ_88, SQ_89])
        );

        assert_eq!(
            king_effect(SQ_81),
            bb_from_sqs([SQ_71, SQ_72, SQ_82, SQ_91, SQ_92])
        );
    }

    #[test]
    fn test_gold_effect() {
        assert_eq!(
            gold_effect(HUM, SQ_85),
            bb_from_sqs([SQ_74, SQ_75, SQ_84, SQ_86, SQ_94, SQ_95])
        );

        assert_eq!(
            gold_effect(COM, SQ_85),
            bb_from_sqs([SQ_75, SQ_76, SQ_84, SQ_86, SQ_95, SQ_96])
        );

        assert_eq!(gold_effect(HUM, SQ_11), bb_from_sqs([SQ_12, SQ_21]));

        assert_eq!(gold_effect(COM, SQ_99), bb_from_sqs([SQ_89, SQ_98]));
    }

    #[test]
    fn test_silver_effect() {
        assert_eq!(
            silver_effect(HUM, SQ_85),
            bb_from_sqs([SQ_74, SQ_76, SQ_84, SQ_94, SQ_96])
        );

        assert_eq!(
            silver_effect(COM, SQ_85),
            bb_from_sqs([SQ_74, SQ_76, SQ_86, SQ_94, SQ_96])
        );

        assert_eq!(silver_effect(HUM, SQ_11), bb_from_sqs([SQ_22]));

        assert_eq!(silver_effect(COM, SQ_99), bb_from_sqs([SQ_88]));
    }

    #[test]
    fn test_knight_effect() {
        assert_eq!(knight_effect(HUM, SQ_85), bb_from_sqs([SQ_73, SQ_93]));

        assert_eq!(knight_effect(COM, SQ_85), bb_from_sqs([SQ_77, SQ_97]));

        assert_eq!(knight_effect(HUM, SQ_13), bb_from_sqs([SQ_21]));

        assert_eq!(knight_effect(COM, SQ_97), bb_from_sqs([SQ_89]));
    }

    #[test]
    fn test_effect() {
        const PKS: [PieceKind; 14] = [
            PAWN, LANCE, KNIGHT, SILVER, BISHOP, ROOK, GOLD, KING, PRO_PAWN, PRO_LANCE, PRO_KNIGHT,
            PRO_SILVER, HORSE, DRAGON,
        ];

        // 空の盤面で駒種 PKS[i] を５五に置いたときの利き数
        const COUNTS_EMPTY: [u32; 14] = [1, 4, 2, 5, 16, 16, 6, 8, 6, 6, 6, 6, 20, 20];

        // 駒が敷き詰められた盤面で駒種 PKS[i] を５五に置いたときの利き数
        const COUNTS_FULL: [u32; 14] = [1, 1, 2, 5, 4, 4, 6, 8, 6, 6, 6, 6, 8, 8];

        for side in Side::iter() {
            for (i, pk) in PKS.into_iter().enumerate() {
                let pc = Piece::new(side, pk);

                assert_eq!(
                    effect(pc, SQ_55, Bitboard::zero()).count_ones(),
                    COUNTS_EMPTY[i]
                );
                assert_eq!(
                    effect(pc, SQ_55, Bitboard::all()).count_ones(),
                    COUNTS_FULL[i]
                );
            }
        }
    }

    #[test]
    fn test_pawn_bb_effect() {
        assert_eq!(
            pawn_bb_effect(HUM, bb_from_sqs([SQ_14, SQ_35, SQ_79, SQ_82])),
            bb_from_sqs([SQ_13, SQ_34, SQ_78, SQ_81])
        );

        assert_eq!(
            pawn_bb_effect(COM, bb_from_sqs([SQ_14, SQ_35, SQ_78, SQ_81])),
            bb_from_sqs([SQ_15, SQ_36, SQ_79, SQ_82])
        );
    }

    #[test]
    fn test_pawn_drop_mask() {
        assert_eq!(
            pawn_drop_mask(HUM, bb_from_sqs([SQ_14, SQ_32, SQ_59, SQ_77, SQ_83, SQ_95])),
            bb_from_sqs(itertools::chain!(
                Square::iter_range(SQ_22, SQ_29),
                Square::iter_range(SQ_42, SQ_49),
                Square::iter_range(SQ_62, SQ_69)
            ))
        );

        assert_eq!(
            pawn_drop_mask(COM, bb_from_sqs([SQ_23, SQ_37, SQ_41, SQ_52, SQ_73])),
            bb_from_sqs(itertools::chain!(
                Square::iter_range(SQ_11, SQ_18),
                Square::iter_range(SQ_61, SQ_68),
                Square::iter_range(SQ_81, SQ_88),
                Square::iter_range(SQ_91, SQ_98)
            ))
        );
    }

    #[test]
    fn test_around25() {
        assert_eq!(
            around25(SQ_78),
            bb_from_sqs([
                SQ_56, SQ_57, SQ_58, SQ_59, SQ_66, SQ_67, SQ_68, SQ_69, SQ_76, SQ_77, SQ_78, SQ_79,
                SQ_86, SQ_87, SQ_88, SQ_89, SQ_96, SQ_97, SQ_98, SQ_99
            ])
        );

        assert_eq!(
            around25(SQ_63),
            bb_from_sqs([
                SQ_41, SQ_42, SQ_43, SQ_44, SQ_45, SQ_51, SQ_52, SQ_53, SQ_54, SQ_55, SQ_61, SQ_62,
                SQ_63, SQ_64, SQ_65, SQ_71, SQ_72, SQ_73, SQ_74, SQ_75, SQ_81, SQ_82, SQ_83, SQ_84,
                SQ_85
            ])
        );
    }
}
