#[allow(unused_imports)]
use pretty_assertions::{assert_eq, assert_ne};

use naitou_clone::*;

/// 平手初期局面からある程度の深さまでについて、駒取りの指し手の個数が既知の結果と一致するかテストする。
/// 少し時間がかかるので `#[ignore]` を付けてある。実行する際はリリースモードにすること。
#[test]
#[ignore]
fn test_generate_captures_startpos() {
    const SFEN: &str = "startpos";

    bbs::init();

    let mut pos = sfen_to_position(SFEN);

    assert_eq!(f_count(&mut pos, 1), 0);
    assert_eq!(f_count(&mut pos, 2), 0);
    assert_eq!(f_count(&mut pos, 3), 59);
    assert_eq!(f_count(&mut pos, 4), 1803);
    assert_eq!(f_count(&mut pos, 5), 113680);
    assert_eq!(f_count(&mut pos, 6), 3387051);
}

/// 合法手が最多(593 手)の局面からある程度の深さまでについて、駒取りの指し手の個数が既知の結果と一致するかテストする。
/// 少し時間がかかるので `#[ignore]` を付けてある。実行する際はリリースモードにすること。
#[test]
#[ignore]
fn test_generate_captures_max_moves() {
    const SFEN: &str = "sfen R8/2K1S1SSk/4B4/9/9/9/9/9/1L1L1L3 b RBGSNLP3g3n17p 1";

    bbs::init();

    let mut pos = sfen_to_position(SFEN);

    assert_eq!(f_count(&mut pos, 1), 0);
    assert_eq!(f_count(&mut pos, 2), 538);
    assert_eq!(f_count(&mut pos, 3), 197899);
    assert_eq!(f_count(&mut pos, 4), 60043133);
}

fn f_count(pos: &mut Position, depth: u32) -> u64 {
    let us = pos.side_to_move();
    let them = us.inv();

    // 手番でない側に王手がかかっていればこの局面は違法。
    if pos.is_checked(them) {
        return 0;
    }

    if depth == 0 {
        // 末端ノードは駒取りの指し手のみで生成されるので、打ち歩詰めは気にしなくてよい。
        return 1;
    }

    // 残り深さ 1 ならば駒取りの指し手のみを生成。さもなくば通常の指し手生成。
    let mvs = if depth == 1 {
        generate_captures(pos)
    } else if pos.is_checked(us) {
        generate_evasions(pos)
    } else {
        generate_moves(pos)
    };

    let mut res = 0;

    for mv in mvs {
        let umv = pos.do_move(mv);
        res += f_count(pos, depth - 1);
        pos.undo_move(umv);
    }

    res
}

fn sfen_to_position(sfen: &str) -> Position {
    let (side_to_move, board, hands) = sfen_decode_position(sfen).unwrap();

    Position::new(side_to_move, board, hands)
}
