//! COM 側の指し手生成。順序が原作通りになっている。

// 原作では、マスを９一, ８一, ..., １九の順に調べ、以下のように指し手を生成する:
//
// * マスが空白なら駒打ちの指し手を生成
// * マスに COM 駒があればそれを動かす指し手を生成
//
// 駒打ちの指し手は歩、香、桂、銀、金、角、飛車の順に生成される。
// 盤上の駒を動かす指し手は駒種ごとに移動先の順序が決まっている。また、成れる場合は必ず成る。

use crate::bbs;
use crate::bitboard::Bitboard;
use crate::naitou::*;
use crate::position::Position;
use crate::shogi::*;

use super::MoveArray;

/// 指定した局面における全ての COM 側の指し手を生成する。順序は原作通りになっている。
///
/// 生成された指し手は違法手も含む。自殺手判定などは思考ルーチン側で行う(必ずしも正しくない)。
///
/// 局面は COM の手番であることを仮定している。
pub fn generate_moves_com(pos: &Position) -> MoveArray {
    debug_assert_eq!(pos.side_to_move(), COM);

    // 盤上の駒の移動先の bitboard を予め求めておく。つまりそれは自駒のないマスである。
    let bb_walk_target = !pos.bb_occupied_side(COM);

    // 歩を打てるマスの bitboard を予め求めておく。
    let bb_pawn_drop = bbs::pawn_drop_mask(COM, pos.bb_piece(COM, PAWN));

    let mut mvs = MoveArray::new();

    // 原作通りの順序でマスを列挙し、空白なら駒打ちを、COM 駒ならそれを動かす手を生成。
    for sq in naitou_squares() {
        match pos.board()[sq] {
            NO_PIECE => generate_moves_com_drop(pos, sq, bb_pawn_drop, &mut mvs),
            pc if pc.side() == COM => {
                generate_moves_com_walk(pos, sq, pc.kind(), bb_walk_target, &mut mvs)
            }
            _ => {}
        }
    }

    mvs
}

/// マス `src` にある COM 駒種 `pk` を動かす指し手を生成する。
fn generate_moves_com_walk(
    pos: &Position,
    src: Square,
    pk: PieceKind,
    bb_target: Bitboard,
    mvs: &mut MoveArray,
) {
    let src_ww = SquareWithWall::from(src);

    // 遠隔利きによる指し手を生成。
    {
        for &delta in effect_ranged(pk) {
            let mut dst_ww = src_ww + delta;
            while dst_ww.is_on_board() && bb_target.test_square(Square::from(dst_ww)) {
                let dst = Square::from(dst_ww);
                generate_walk_helper(pk, src, dst, mvs);

                // HUM 駒にぶつかったらこの方向は打ち切り。
                if pos.board()[dst] != NO_PIECE {
                    break;
                }

                dst_ww += delta;
            }
        }
    }

    // 近接利きによる指し手を生成。
    {
        for &delta in effect_melee(pk) {
            let dst_ww = src_ww + delta;
            if dst_ww.is_on_board() && bb_target.test_square(Square::from(dst_ww)) {
                let dst = Square::from(dst_ww);
                generate_walk_helper(pk, src, dst, mvs);
            }
        }
    }
}

/// 駒種 `pk` を `src` から `dst` へ動かす指し手を生成する。成れる場合は必ず成る。
fn generate_walk_helper(pk: PieceKind, src: Square, dst: Square, mvs: &mut MoveArray) {
    let promo = pk.is_promotable() && (src.is_promotion_zone(COM) || dst.is_promotion_zone(COM));

    if promo {
        mvs.push(Move::new_walk_promotion(src, dst));
    } else {
        mvs.push(Move::new_walk(src, dst));
    }
}

/// 指定した駒種の近接利き方向たちを返す。
const fn effect_melee(pk: PieceKind) -> &'static [i32] {
    const EFF_PAWN: &[i32] = &[SquareWithWall::DIR_D];

    const EFF_KNIGHT: &[i32] = &[SquareWithWall::DIR_RDD, SquareWithWall::DIR_LDD];

    const EFF_SILVER: &[i32] = &[
        SquareWithWall::DIR_RD,
        SquareWithWall::DIR_D,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_RU,
        SquareWithWall::DIR_LU,
    ];

    const EFF_GOLD: &[i32] = &[
        SquareWithWall::DIR_RD,
        SquareWithWall::DIR_D,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_R,
        SquareWithWall::DIR_L,
        SquareWithWall::DIR_U,
    ];

    const EFF_KING: &[i32] = &[
        SquareWithWall::DIR_RD,
        SquareWithWall::DIR_D,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_R,
        SquareWithWall::DIR_L,
        SquareWithWall::DIR_RU,
        SquareWithWall::DIR_U,
        SquareWithWall::DIR_LU,
    ];

    const EFF_HORSE: &[i32] = &[
        SquareWithWall::DIR_D,
        SquareWithWall::DIR_R,
        SquareWithWall::DIR_L,
        SquareWithWall::DIR_U,
    ];

    const EFF_DRAGON: &[i32] = &[
        SquareWithWall::DIR_RD,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_RU,
        SquareWithWall::DIR_LU,
    ];

    const TABLE: [&[i32]; 15] = [
        &[],        // NO_PIECE_KIND
        EFF_PAWN,   // PAWN
        &[],        // LANCE
        EFF_KNIGHT, // KNIGHT
        EFF_SILVER, // SILVER
        &[],        // BISHOP
        &[],        // ROOK
        EFF_GOLD,   // GOLD
        EFF_KING,   // KING
        EFF_GOLD,   // PRO_PAWN
        EFF_GOLD,   // PRO_LANCE
        EFF_GOLD,   // PRO_KNIGHT
        EFF_GOLD,   // PRO_SILVER
        EFF_HORSE,  // HORSE
        EFF_DRAGON, // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// 指定した駒種の遠隔利き方向たちを返す。
const fn effect_ranged(pk: PieceKind) -> &'static [i32] {
    const EFF_LANCE: &[i32] = &[SquareWithWall::DIR_D];

    const EFF_BISHOP: &[i32] = &[
        SquareWithWall::DIR_LU,
        SquareWithWall::DIR_RU,
        SquareWithWall::DIR_LD,
        SquareWithWall::DIR_RD,
    ];

    const EFF_ROOK: &[i32] = &[
        SquareWithWall::DIR_U,
        SquareWithWall::DIR_D,
        SquareWithWall::DIR_L,
        SquareWithWall::DIR_R,
    ];

    const TABLE: [&[i32]; 15] = [
        &[],        // NO_PIECE_KIND
        &[],        // PAWN
        EFF_LANCE,  // LANCE
        &[],        // KNIGHT
        &[],        // SILVER
        EFF_BISHOP, // BISHOP
        EFF_ROOK,   // ROOK
        &[],        // GOLD
        &[],        // KING
        &[],        // PRO_PAWN
        &[],        // PRO_LANCE
        &[],        // PRO_KNIGHT
        &[],        // PRO_SILVER
        EFF_BISHOP, // HORSE
        EFF_ROOK,   // DRAGON
    ];

    TABLE[pk.inner() as usize]
}

/// マス `dst` を対象とする駒打ちの指し手を生成する。
fn generate_moves_com_drop(
    pos: &Position,
    dst: Square,
    bb_pawn_drop: Bitboard,
    mvs: &mut MoveArray,
) {
    let us = pos.side_to_move();
    let hand = pos.hand(us);

    // 歩、香、桂、銀、金、角、飛車の順に生成。

    // 歩の場合、二歩と敵陣 1 段目を弾く。
    if hand[PAWN] > 0 && bb_pawn_drop.test_square(dst) {
        mvs.push(Move::new_drop(PAWN, dst));
    }

    // 香の場合、敵陣 1 段目を弾く。
    if hand[LANCE] > 0 && dst.row() != ROW_9 {
        mvs.push(Move::new_drop(LANCE, dst));
    }

    // 桂の場合、敵陣 1, 2 段目を弾く。
    if hand[KNIGHT] > 0 && dst.row() <= ROW_7 {
        mvs.push(Move::new_drop(KNIGHT, dst));
    }

    // 他の駒種は制限なし。
    if hand[SILVER] > 0 {
        mvs.push(Move::new_drop(SILVER, dst));
    }
    if hand[GOLD] > 0 {
        mvs.push(Move::new_drop(GOLD, dst));
    }
    if hand[BISHOP] > 0 {
        mvs.push(Move::new_drop(BISHOP, dst));
    }
    if hand[ROOK] > 0 {
        mvs.push(Move::new_drop(ROOK, dst));
    }
}
