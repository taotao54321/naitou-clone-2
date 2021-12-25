#[allow(unused_imports)]
use pretty_assertions::{assert_eq, assert_ne};

use naitou_clone::*;

#[derive(Debug, Eq, PartialEq)]
struct PerftStats {
    count_all: u64,
    count_capture: u64,
    count_promote: u64,
    count_check: u64,
    count_checkmate: u64,
}

impl PerftStats {
    fn new() -> Self {
        Self {
            count_all: 0,
            count_capture: 0,
            count_promote: 0,
            count_check: 0,
            count_checkmate: 0,
        }
    }
}

fn do_perft(pos: &mut Position, depth: u32) -> PerftStats {
    let mut stats = PerftStats::new();

    perft(pos, depth, |leaf| {
        stats.count_all += 1;

        if let Some(umv) = leaf.previous_move() {
            if !umv.is_drop() {
                if umv.piece_captured() != NO_PIECE {
                    stats.count_capture += 1;
                }
                if umv.is_promotion() {
                    stats.count_promote += 1;
                }
            }
        }

        if leaf.is_checked() {
            stats.count_check += 1;
        }

        if leaf.is_checkmated() {
            stats.count_checkmate += 1;
        }
    });

    stats
}

/// 平手初期局面からの perft が既知の結果と一致するかテストする。
/// 少し時間がかかるので `#[ignore]` を付けてある。実行する際はリリースモードにすること。
///
/// ref:
///
/// * [将棋でPerftしてみたまとめ - Qiita](https://qiita.com/ak11/items/8bd5f2bb0f5b014143c8)
/// * [将棋でPerftしてみたまとめのまとめ | やねうら王 公式サイト](https://yaneuraou.yaneu.com/2015/12/13/%E5%B0%86%E6%A3%8B%E3%81%A7perft%E3%81%97%E3%81%A6%E3%81%BF%E3%81%9F%E3%81%BE%E3%81%A8%E3%82%81%E3%81%AE%E3%81%BE%E3%81%A8%E3%82%81/)
#[test]
#[ignore]
fn test_perft_startpos() {
    const SFEN: &str = "startpos";

    // 統合テストではライブラリ側の #[cfg(test)] は無効になるので、
    // リリースモードでテストする場合は初期化が必要。
    bbs::init();

    let (side_to_move, board, hands) = sfen_decode_position(SFEN).unwrap();
    let mut pos = Position::new(side_to_move, board, hands);

    assert_eq!(
        do_perft(&mut pos, 1),
        PerftStats {
            count_all: 30,
            count_capture: 0,
            count_promote: 0,
            count_check: 0,
            count_checkmate: 0,
        }
    );

    assert_eq!(
        do_perft(&mut pos, 2),
        PerftStats {
            count_all: 900,
            count_capture: 0,
            count_promote: 0,
            count_check: 0,
            count_checkmate: 0,
        }
    );

    assert_eq!(
        do_perft(&mut pos, 3),
        PerftStats {
            count_all: 25470,
            count_capture: 59,
            count_promote: 30,
            count_check: 48,
            count_checkmate: 0,
        }
    );

    assert_eq!(
        do_perft(&mut pos, 4),
        PerftStats {
            count_all: 719731,
            count_capture: 1803,
            count_promote: 842,
            count_check: 1121,
            count_checkmate: 0,
        }
    );

    assert_eq!(
        do_perft(&mut pos, 5),
        PerftStats {
            count_all: 19861490,
            count_capture: 113680,
            count_promote: 57214,
            count_check: 71434,
            count_checkmate: 0,
        }
    );
}

/// 合法手が最多(593 手)の局面からの perft が既知の結果と一致するかテストする。
/// 少し時間がかかるので `#[ignore]` を付けてある。実行する際はリリースモードにすること。
///
/// ref:
///
/// * [将棋でPerftしてみたまとめ - Qiita](https://qiita.com/ak11/items/8bd5f2bb0f5b014143c8)
#[test]
#[ignore]
fn test_perft_max_moves() {
    const SFEN: &str = "sfen R8/2K1S1SSk/4B4/9/9/9/9/9/1L1L1L3 b RBGSNLP3g3n17p 1";

    // 統合テストではライブラリ側の #[cfg(test)] は無効になるので、
    // リリースモードでテストする場合は初期化が必要。
    bbs::init();

    let (side_to_move, board, hands) = sfen_decode_position(SFEN).unwrap();
    let mut pos = Position::new(side_to_move, board, hands);

    assert_eq!(
        do_perft(&mut pos, 1),
        PerftStats {
            count_all: 593,
            count_capture: 0,
            count_promote: 52,
            count_check: 40,
            count_checkmate: 6,
        }
    );

    assert_eq!(
        do_perft(&mut pos, 2),
        PerftStats {
            count_all: 105677,
            count_capture: 538,
            count_promote: 0,
            count_check: 3802,
            count_checkmate: 0,
        }
    );

    // これは打ち歩詰め判定をサボっていると失敗する。
    assert_eq!(
        do_perft(&mut pos, 3),
        PerftStats {
            count_all: 53393368,
            count_capture: 197899,
            count_promote: 4875102,
            count_check: 3493971,
            count_checkmate: 566203,
        }
    );
}
