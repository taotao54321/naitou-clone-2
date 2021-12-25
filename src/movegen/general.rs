//! 通常の指し手生成。
//! 生成される指し手の順序は未規定。HUM 側の指し手生成に使うことを想定している。

use crate::bbs;
use crate::bitboard::Bitboard;
use crate::position::Position;
use crate::shogi::*;

use super::MoveArray;

/// 指定した局面における全ての疑似合法手を生成する。
///
/// 疑似合法手には、通常の将棋の合法手に加え、以下が含まれる:
///
/// * 自殺手
/// * 打ち歩詰め
/// * 連続王手の千日手
///
/// ここでは MoveArray を直接返す (return value optimization を期待している)。
pub fn generate_moves(pos: &Position) -> MoveArray {
    let us = pos.side_to_move();

    let mut mvs = MoveArray::new();

    // 盤上の駒の移動先の bitboard を得る。つまりそれは自駒のないマスである。
    let bb_target = !pos.bb_occupied_side(us);

    generate_moves_walk_pawn(pos, bb_target, &mut mvs);

    generate_moves_walk(pos, bb_target, LANCE, &mut mvs);
    generate_moves_walk(pos, bb_target, KNIGHT, &mut mvs);
    generate_moves_walk(pos, bb_target, SILVER, &mut mvs);
    generate_moves_walk(pos, bb_target, BISHOP, &mut mvs);
    generate_moves_walk(pos, bb_target, ROOK, &mut mvs);

    generate_moves_walk(pos, bb_target, GOLD, &mut mvs);
    generate_moves_walk(pos, bb_target, KING, &mut mvs);
    generate_moves_walk(pos, bb_target, PRO_PAWN, &mut mvs);
    generate_moves_walk(pos, bb_target, PRO_LANCE, &mut mvs);
    generate_moves_walk(pos, bb_target, PRO_KNIGHT, &mut mvs);
    generate_moves_walk(pos, bb_target, PRO_SILVER, &mut mvs);
    generate_moves_walk(pos, bb_target, HORSE, &mut mvs);
    generate_moves_walk(pos, bb_target, DRAGON, &mut mvs);

    // drop の対象になるのは空白マスのみ。
    generate_moves_drop(pos, pos.bb_blank(), &mut mvs);

    mvs
}

/// 盤上の歩を動かす指し手を生成する。
#[inline]
fn generate_moves_walk_pawn(pos: &Position, bb_target: Bitboard, mvs: &mut MoveArray) {
    let us = pos.side_to_move();

    // 歩の場合、全ての利きを一気に求め、それをそのまま指し手生成に使える。
    // 敵陣一段目の歩は存在しないと仮定している。
    let bb_pawn = pos.bb_piece(us, PAWN);
    let bb_eff = bbs::pawn_bb_effect(us, bb_pawn);

    // 全ての歩の利きを bb_target でマスクすれば可能な指し手の移動先が得られる。
    let bb_dst = bb_target & bb_eff;

    // 敵陣 1 段目を得る。
    let row_deadend = if us == HUM { ROW_1 } else { ROW_9 };

    bb_dst.for_each_square(|dst| {
        let src = dst
            + if us == HUM {
                Square::DIR_D
            } else {
                Square::DIR_U
            };

        if dst.is_promotion_zone(us) {
            // dst が敵陣なら必ず成れる。
            // dst が敵陣 1 段目でなければ不成も可能。
            mvs.push(Move::new_walk_promotion(src, dst));
            if dst.row() != row_deadend {
                mvs.push(Move::new_walk(src, dst));
            }
        } else {
            // dst が敵陣でなければ常に不成が可能。
            mvs.push(Move::new_walk(src, dst));
        }
    });
}

/// 盤上の歩以外の駒を動かす指し手を生成する。
#[inline]
fn generate_moves_walk(pos: &Position, bb_target: Bitboard, pk: PieceKind, mvs: &mut MoveArray) {
    debug_assert_ne!(pk, PAWN);

    let us = pos.side_to_move();

    let bb_occ = pos.bb_occupied();
    let bb_pc = pos.bb_piece(us, pk);

    bb_pc.for_each_square(|src| {
        let bb_dst = bb_target & bbs::effect(Piece::new(us, pk), src, bb_occ);

        match pk {
            LANCE => generate_moves_walk_helper_lance(us, src, bb_dst, mvs),
            KNIGHT => generate_moves_walk_helper_knight(us, src, bb_dst, mvs),
            SILVER | BISHOP | ROOK => generate_moves_walk_helper_promotable(us, src, bb_dst, mvs),
            GOLD | KING | PRO_PAWN | PRO_LANCE | PRO_KNIGHT | PRO_SILVER | HORSE | DRAGON => {
                generate_moves_walk_helper_unpromotable(src, bb_dst, mvs)
            }
            _ => unreachable!(),
        }
    });
}

/// 移動元と移動先候補を決めた上で、香を動かす指し手を生成する。
#[inline]
fn generate_moves_walk_helper_lance(us: Side, src: Square, bb_dst: Bitboard, mvs: &mut MoveArray) {
    // まず成りを生成する。bb_dst のうち敵陣のマスに対しては常に成りが可能。
    // 香の場合、敵陣から自陣に出るケースはありえない。
    {
        let bb_dst_promo = bb_dst & bbs::promotion_zone(us);

        bb_dst_promo.for_each_square(|dst| {
            mvs.push(Move::new_walk_promotion(src, dst));
        });
    }

    // 次に不成を生成する。bb_dst のうち敵陣 1 段目を除いたマスに対して不成が可能。
    {
        let bb_mask_nonpromo = if us == HUM {
            bbs::forward_rows(COM, ROW_1)
        } else {
            bbs::forward_rows(HUM, ROW_9)
        };
        let bb_dst_nonpromo = bb_dst & bb_mask_nonpromo;

        bb_dst_nonpromo.for_each_square(|dst| {
            mvs.push(Move::new_walk(src, dst));
        });
    }
}

/// 移動元と移動先候補を決めた上で、桂を動かす指し手を生成する。
#[inline]
fn generate_moves_walk_helper_knight(us: Side, src: Square, bb_dst: Bitboard, mvs: &mut MoveArray) {
    bb_dst.for_each_square(|dst| {
        if dst.is_promotion_zone(us) {
            // dst が敵陣なら必ず成れる。
            // dst が敵陣 3 段目ならば不成も可能。
            mvs.push(Move::new_walk_promotion(src, dst));
            let row_nonpromo = if us == HUM { ROW_3 } else { ROW_7 };
            if dst.row() == row_nonpromo {
                mvs.push(Move::new_walk(src, dst));
            }
        } else {
            // dst が敵陣でなければ常に不成が可能。
            mvs.push(Move::new_walk(src, dst));
        }
    });
}

/// 移動元と移動先候補を決めた上で、成れる駒(歩、香、桂を除く)を動かす指し手を生成する。
#[inline]
fn generate_moves_walk_helper_promotable(
    us: Side,
    src: Square,
    bb_dst: Bitboard,
    mvs: &mut MoveArray,
) {
    // 歩、香、桂は除いているので、行きどころのない駒は生じない。

    if src.is_promotion_zone(us) {
        // 移動元が敵陣なら成り、不成どちらも常に可能。
        bb_dst.for_each_square(|dst| {
            mvs.push(Move::new_walk_promotion(src, dst));
            mvs.push(Move::new_walk(src, dst));
        });
    } else {
        // 移動元が敵陣でない場合、敵陣への移動のみで成れる。不成は常に可能。
        {
            let bb_dst_promo = bb_dst & bbs::promotion_zone(us);
            bb_dst_promo.for_each_square(|dst| {
                mvs.push(Move::new_walk_promotion(src, dst));
                mvs.push(Move::new_walk(src, dst));
            });
        }
        {
            let bb_dst_nonpromo = bbs::promotion_zone(us).andnot(bb_dst);
            bb_dst_nonpromo.for_each_square(|dst| {
                mvs.push(Move::new_walk(src, dst));
            });
        }
    }
}

/// 移動元と移動先候補を決めた上で、成れない駒を動かす指し手を生成する。
#[inline]
fn generate_moves_walk_helper_unpromotable(src: Square, bb_dst: Bitboard, mvs: &mut MoveArray) {
    bb_dst.for_each_square(|dst| {
        mvs.push(Move::new_walk(src, dst));
    });
}

/// 移動先候補を決めた上で、駒打ちの指し手を生成する。
#[inline]
fn generate_moves_drop(pos: &Position, bb_target: Bitboard, mvs: &mut MoveArray) {
    let us = pos.side_to_move();
    let hand = pos.hand(us);

    if hand[PAWN] > 0 {
        generate_moves_drop_pawn(pos, bb_target, mvs);
    }
    if hand[LANCE] > 0 {
        generate_moves_drop_lance(us, bb_target, mvs);
    }
    if hand[KNIGHT] > 0 {
        generate_moves_drop_knight(us, bb_target, mvs);
    }

    for pk in [SILVER, GOLD, BISHOP, ROOK] {
        if hand[pk] == 0 {
            continue;
        }
        bb_target.for_each_square(|dst| {
            mvs.push(Move::new_drop(pk, dst));
        });
    }
}

/// 歩を打つ指し手を生成する。
#[inline]
fn generate_moves_drop_pawn(pos: &Position, bb_target: Bitboard, mvs: &mut MoveArray) {
    // 二歩と敵陣 1 段目の歩を弾く。
    let us = pos.side_to_move();
    let bb_dst = bb_target & bbs::pawn_drop_mask(us, pos.bb_piece(us, PAWN));

    bb_dst.for_each_square(|dst| {
        mvs.push(Move::new_drop(PAWN, dst));
    });
}

/// 香を打つ指し手を生成する。
#[inline]
fn generate_moves_drop_lance(us: Side, bb_target: Bitboard, mvs: &mut MoveArray) {
    // 敵陣 1 段目の香を弾く。
    let bb_mask = if us == HUM {
        bbs::forward_rows(COM, ROW_1)
    } else {
        bbs::forward_rows(HUM, ROW_9)
    };
    let bb_dst = bb_target & bb_mask;

    bb_dst.for_each_square(|dst| {
        mvs.push(Move::new_drop(LANCE, dst));
    });
}

/// 桂を打つ指し手を生成する。
#[inline]
fn generate_moves_drop_knight(us: Side, bb_target: Bitboard, mvs: &mut MoveArray) {
    // 敵陣 1, 2 段目の桂を弾く。
    let bb_mask = if us == HUM {
        bbs::forward_rows(COM, ROW_2)
    } else {
        bbs::forward_rows(HUM, ROW_8)
    };
    let bb_dst = bb_target & bb_mask;

    bb_dst.for_each_square(|dst| {
        mvs.push(Move::new_drop(KNIGHT, dst));
    });
}

/// 指定した局面における全ての疑似王手回避手(必ずしも王手を回避しない)を生成する。
/// 手番の側に王手がかかっていることを仮定している。
///
/// 原作通りの詰み判定を行う際は `generate_evasions_naitou()` を使うこと。
#[inline]
pub fn generate_evasions(pos: &Position) -> MoveArray {
    // 全ての空白マスが駒打ちの対象となる。
    let bb_drop_target = pos.bb_blank();

    generate_evasions_impl(pos, bb_drop_target)
}

/// 指定した局面における原作通りの疑似王手回避手(必ずしも王手を回避しない)を生成する。
/// 局面が HUM の手番であり、HUM 玉に王手がかかっていることを仮定している。
///
/// `generate_evasions()` と異なり、駒打ちは玉周り 8 マスのみを対象とする。
pub fn generate_evasions_naitou(pos: &Position) -> MoveArray {
    debug_assert_eq!(pos.side_to_move(), HUM);

    // 空白マスのうち、玉周り 8 マスのみが駒打ちの対象となる。
    let bb_drop_target = pos.bb_blank() & bbs::king_effect(pos.king_square(HUM));

    generate_evasions_impl(pos, bb_drop_target)
}

/// 王手回避手生成ルーチン本体。駒打ち対象マスとして `bb_drop_target` を与える。
/// 手番の側に王手がかかっていることを仮定している。
fn generate_evasions_impl(pos: &Position, bb_drop_target: Bitboard) -> MoveArray {
    // ガチでやると実装が結構重そうなので適当にサボる:
    //
    // * 玉を動かす候補手を生成。
    //   敵の利きへの移動は生成しないが、移動と逆方向からの利きは見落としてしまう。
    // * 玉を動かす以外の候補手を生成。
    //   - 桂で王手されていれば、その桂の位置のみを移動先候補とする。
    //   - さもなくば自玉から見たクイーンの利きのみを移動先候補とする。

    let us = pos.side_to_move();
    let them = us.inv();
    let sq_king = pos.king_square(us);

    debug_assert!(pos.is_checked(us));

    let mut mvs = MoveArray::new();

    // 玉を動かす手を生成。

    // 自駒のないマスのうち、自玉の利きがあるマスのみが候補。
    // (敵の利きがあるかどうかは後で判定する)
    let bb_dst_king = !pos.bb_occupied_side(us) & bbs::king_effect(sq_king);
    generate_evasions_king(pos, sq_king, bb_dst_king, &mut mvs);

    // 玉を動かす以外の候補手を生成。必ずしも王手回避になるとは限らない。

    // 桂で王手されているか?
    let bb_knight = pos.bb_piece(them, KNIGHT) & bbs::knight_effect(us, sq_king);

    // 移動先候補を求める。
    let bb_target = if bb_knight.is_zero() {
        // 桂で王手されていなければ、自駒のないマスのうち、自玉から見たクイーンの利きのみが候補。
        !pos.bb_occupied_side(us) & bbs::queen_effect(sq_king, pos.bb_occupied())
    } else {
        // 桂で王手されていれば、その桂の位置のみが候補。
        bb_knight
    };

    // 候補手生成。
    generate_evasions_nonking(pos, bb_target, bb_drop_target, &mut mvs);

    mvs
}

/// 玉を動かす以外の疑似王手回避手生成。
#[inline]
fn generate_evasions_nonking(
    pos: &Position,
    bb_target: Bitboard,
    bb_drop_target: Bitboard,
    mvs: &mut MoveArray,
) {
    generate_moves_walk_pawn(pos, bb_target, mvs);

    generate_moves_walk(pos, bb_target, LANCE, mvs);
    generate_moves_walk(pos, bb_target, KNIGHT, mvs);
    generate_moves_walk(pos, bb_target, SILVER, mvs);
    generate_moves_walk(pos, bb_target, BISHOP, mvs);
    generate_moves_walk(pos, bb_target, ROOK, mvs);

    generate_moves_walk(pos, bb_target, GOLD, mvs);
    generate_moves_walk(pos, bb_target, PRO_PAWN, mvs);
    generate_moves_walk(pos, bb_target, PRO_LANCE, mvs);
    generate_moves_walk(pos, bb_target, PRO_KNIGHT, mvs);
    generate_moves_walk(pos, bb_target, PRO_SILVER, mvs);
    generate_moves_walk(pos, bb_target, HORSE, mvs);
    generate_moves_walk(pos, bb_target, DRAGON, mvs);

    // 駒打ちは指定されたマスのみを対象とする。
    // 桂の王手に対しては駒打ちは無意味なので、bb_target との AND をとる。
    generate_moves_drop(pos, bb_target & bb_drop_target, mvs);
}

/// 玉を動かす疑似王手回避手生成。
///
/// 敵の利きへの移動は生成しないが、この判定には `Position::effect_bount_board()` を使っているため、
/// 移動と逆方向からの利きは見落としてしまう。たとえば:
///
/// ```text
/// v歩 玉v飛
/// ```
///
/// この局面で玉を左に動かす手も生成される。
#[inline]
fn generate_evasions_king(pos: &Position, src: Square, bb_dst: Bitboard, mvs: &mut MoveArray) {
    let us = pos.side_to_move();
    let them = us.inv();

    bb_dst.for_each_square(|dst| {
        // 敵の利きがあるマスには行けない。
        if pos.effect_count_board(them)[dst] > 0 {
            return;
        }
        mvs.push(Move::new_walk(src, dst));
    });
}

/// 指定した局面で手番の側がチェックメイト(**打ち歩含む**)されているかどうかを返す。
/// 手番の側に王手がかかっていることを仮定している。
///
/// 関数から戻ったとき、`pos` は呼び出し前の局面に戻っている。
#[inline]
pub fn position_is_checkmated(pos: &mut Position) -> bool {
    // 全ての空白マスが駒打ちの対象となる。
    let bb_drop_target = pos.bb_blank();

    position_is_checkmated_impl(pos, bb_drop_target)
}

/// 指定した局面で HUM 玉がチェックメイト(**打ち歩含む**)されているかどうかを返す(原作通りの判定)。
/// 局面が HUM の手番であり、HUM 玉に王手がかかっていることを仮定している。
///
/// `position_is_checkmated()` と異なり、駒打ちは玉周り 8 マスのみを対象とする。
/// XXX: これは厳密には正しくない(HUM 玉が入玉しているとき、本来不詰のものが詰みと判定されうる)。
///
/// 関数から戻ったとき、`pos` は呼び出し前の局面に戻っている。
#[inline]
pub fn position_is_checkmated_naitou(pos: &mut Position) -> bool {
    debug_assert_eq!(pos.side_to_move(), HUM);

    // 空白マスのうち、玉周り 8 マスのみが駒打ちの対象となる。
    let bb_drop_target = pos.bb_blank() & bbs::king_effect(pos.king_square(HUM));

    position_is_checkmated_impl(pos, bb_drop_target)
}

/// チェックメイト判定ルーチン本体。駒打ち対象マスとして `bb_drop_target` を与える。
/// 手番の側に王手がかかっていることを仮定している。
#[inline]
fn position_is_checkmated_impl(pos: &mut Position, bb_drop_target: Bitboard) -> bool {
    // generate_evasions_impl() と同様だが、王手回避手が見つかったら途中で打ち切る。

    let us = pos.side_to_move();

    debug_assert!(pos.is_checked(us));

    let can_evade = evade_by_king(pos) || evade_by_nonking(pos, bb_drop_target);

    !can_evade
}

/// 手番の側が玉を動かす手で王手回避できるかどうかを返す。
#[inline]
fn evade_by_king(pos: &mut Position) -> bool {
    let us = pos.side_to_move();
    let them = us.inv();
    let sq_king = pos.king_square(us);

    let mut bb_dst = !pos.bb_occupied_side(us) & bbs::king_effect(sq_king);

    while !bb_dst.is_zero() {
        let dst = bb_dst.pop_least_square();

        // 敵の利きがあるマスには行けない。
        if pos.effect_count_board(them)[dst] > 0 {
            continue;
        }

        if try_evade_helper(pos, Move::new_walk(sq_king, dst)) {
            return true;
        }
    }

    false
}

/// 手番の側が玉を動かす以外の手で王手回避できるかどうかを返す。
#[inline]
fn evade_by_nonking(pos: &mut Position, bb_drop_target: Bitboard) -> bool {
    let us = pos.side_to_move();
    let them = us.inv();
    let sq_king = pos.king_square(us);

    let bb_knight = pos.bb_piece(them, KNIGHT) & bbs::knight_effect(us, sq_king);

    let bb_target = if bb_knight.is_zero() {
        !pos.bb_occupied_side(us) & bbs::queen_effect(sq_king, pos.bb_occupied())
    } else {
        bb_knight
    };

    let mut mvs = MoveArray::new();

    macro_rules! try_evade {
        ($stmt:stmt) => {{
            mvs.clear();
            $stmt
            if mvs.iter().any(|&mv| try_evade_helper(pos, mv)) {
                return true;
            }
        }};
    }

    try_evade!(generate_moves_walk_pawn(pos, bb_target, &mut mvs));

    try_evade!(generate_moves_walk(pos, bb_target, LANCE, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, KNIGHT, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, SILVER, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, BISHOP, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, ROOK, &mut mvs));

    try_evade!(generate_moves_walk(pos, bb_target, GOLD, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, PRO_PAWN, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, PRO_LANCE, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, PRO_KNIGHT, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, PRO_SILVER, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, HORSE, &mut mvs));
    try_evade!(generate_moves_walk(pos, bb_target, DRAGON, &mut mvs));

    // 駒打ちは指定されたマスのみを対象とする。
    // 桂の王手に対しては駒打ちは無意味なので、bb_target との AND をとる。
    try_evade!(generate_moves_drop(
        pos,
        bb_target & bb_drop_target,
        &mut mvs
    ));

    false
}

/// 手番の側が指定した指し手で王手回避できるかどうかを返す。
#[inline]
fn try_evade_helper(pos: &mut Position, mv: Move) -> bool {
    let us = pos.side_to_move();

    let umv = pos.do_move(mv);

    let res = !pos.is_checked(us);

    pos.undo_move(umv);

    res
}
