//! 原作固有の要素。

use crate::book::Formation;
use crate::effect::EffectCountBoard;
use crate::engine::{LeafEvaluation, RootEvaluation};
use crate::myarray::*;
use crate::naitou::*;
use crate::shogi::*;

use super::apsp;
use super::backend::{memory_read, Buttons, BUTTONS_A, BUTTONS_D, BUTTONS_S, BUTTONS_T};

/// 盤面または HUM 側の手駒を指すカーソル。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Cursor {
    Board(Square),
    Hand(PieceKind),
}

impl Cursor {
    /// 盤面上のマス `sq` を指すカーソルを返す。
    pub fn new_board(sq: Square) -> Self {
        debug_assert!(sq.is_on_board());

        Self::Board(sq)
    }

    /// HUM 側の手駒 `pk` を指すカーソルを返す。
    pub fn new_hand(pk: PieceKind) -> Self {
        debug_assert!(pk.is_hand());

        Self::Hand(pk)
    }
}

/// 現在の手数を読み取る。
pub fn read_ply() -> u32 {
    let lo = memory_read(0x15);
    let hi = memory_read(0x16);

    100 * u32::from(hi) + u32::from(lo)
}

/// 現在のカーソルを読み取る。
pub fn read_cursor() -> Cursor {
    let x = memory_read(0xD6);
    let y = memory_read(0xD7);

    match (x, y) {
        (1..=9, 1..=9) => {
            let value = 11 * y + x;
            Cursor::new_board(naitou_square_from_value(value).expect("cursor should be valid"))
        }
        (10, 3) => Cursor::new_hand(ROOK),
        (10, 4) => Cursor::new_hand(BISHOP),
        (10, 5) => Cursor::new_hand(GOLD),
        (10, 6) => Cursor::new_hand(SILVER),
        (10, 7) => Cursor::new_hand(KNIGHT),
        (10, 8) => Cursor::new_hand(LANCE),
        (10, 9) => Cursor::new_hand(PAWN),
        _ => panic!("invalid cursor: x={}, y={}", x, y),
    }
}

/// 現在の手番を読み取る。
pub fn read_side_to_move() -> Side {
    if memory_read(0x77) == 0 {
        COM
    } else {
        HUM
    }
}

/// 現在の盤面 A を読み取る。
pub fn read_board_a() -> Board {
    read_board_impl(0x02A9, 0x0322)
}

/// 現在の盤面 B を読み取る。
pub fn read_board_b() -> Board {
    read_board_impl(0x03A9, 0x049B)
}

fn read_board_impl(addr_hum: u16, addr_com: u16) -> Board {
    let mut board = Board::empty();

    for sq in Square::iter() {
        let sq_value = naitou_square_to_value(sq);
        let pc_hum_value = memory_read(addr_hum + u16::from(sq_value));
        let pc_com_value = memory_read(addr_com + u16::from(sq_value));

        let pc_hum = naitou_piece_from_value(pc_hum_value).expect("invalid HUM piece");
        let pc_com = naitou_piece_from_value(pc_com_value).expect("invalid COM piece");

        let pc = match (pc_hum, pc_com) {
            (NO_PIECE, NO_PIECE) => NO_PIECE,
            (NO_PIECE, pc_com) => pc_com,
            (pc_hum, NO_PIECE) => pc_hum,
            (_, _) => panic!(
                "invalid board cell: HUM={}, COM={}",
                pc_hum.kind(),
                pc_com.kind()
            ),
        };

        board[sq] = pc;
    }

    board
}

/// 現在の両陣営の手駒 A を読み取る。
pub fn read_hands_a() -> Hands {
    read_hands_impl(0x039B, 0x03A2)
}

/// 現在の両陣営の手駒 B を読み取る。
pub fn read_hands_b() -> Hands {
    read_hands_impl(0x058D, 0x0594)
}

#[allow(clippy::identity_op)]
fn read_hands_impl(addr_hum: u16, addr_com: u16) -> Hands {
    let mut hands = Hands::from([Hand::empty(); 2]);

    hands[HUM][ROOK] = u32::from(memory_read(addr_hum + 0));
    hands[HUM][BISHOP] = u32::from(memory_read(addr_hum + 1));
    hands[HUM][GOLD] = u32::from(memory_read(addr_hum + 2));
    hands[HUM][SILVER] = u32::from(memory_read(addr_hum + 3));
    hands[HUM][KNIGHT] = u32::from(memory_read(addr_hum + 4));
    hands[HUM][LANCE] = u32::from(memory_read(addr_hum + 5));
    hands[HUM][PAWN] = u32::from(memory_read(addr_hum + 6));

    hands[COM][ROOK] = u32::from(memory_read(addr_com + 0));
    hands[COM][BISHOP] = u32::from(memory_read(addr_com + 1));
    hands[COM][GOLD] = u32::from(memory_read(addr_com + 2));
    hands[COM][SILVER] = u32::from(memory_read(addr_com + 3));
    hands[COM][KNIGHT] = u32::from(memory_read(addr_com + 4));
    hands[COM][LANCE] = u32::from(memory_read(addr_com + 5));
    hands[COM][PAWN] = u32::from(memory_read(addr_com + 6));

    hands
}

/// 指定した陣営について、現在の `EffectCountBoard` を読み取る。
pub fn read_effect_count_board(side: Side) -> EffectCountBoard {
    let addr = if side == HUM { 0x0422 } else { 0x0514 };

    let mut ecb = EffectCountBoard::empty();

    for sq in Square::iter() {
        let sq_value = naitou_square_to_value(sq) as u16;

        ecb[sq] = memory_read(addr + sq_value);
    }

    ecb
}

/// 現在の HUM 側の指し手を読み取る。
pub fn read_move_hum() -> Move {
    read_move_hum_impl(0x05A2, 0x05A1, 0x05BF)
}

/// 現在の COM 側の指し手を読み取る。
pub fn read_move_com() -> Move {
    read_move_com_impl(0x05BC, 0x05BB, 0x05C0)
}

/// 現在思考中の候補手を読み取る。
pub fn read_move_cand() -> Move {
    read_move_com_impl(0x0277, 0x0276, 0x0279)
}

/// 現在思考中の最善手を読み取る。
pub fn read_move_best() -> Option<Move> {
    (read_best_evaluation().disadv_price != 99).then(|| read_move_com_impl(0x0285, 0x0284, 0x028C))
}

fn read_move_hum_impl(addr_src: u16, addr_dst: u16, addr_promo: u16) -> Move {
    let src_value = memory_read(addr_src);
    let dst_value = memory_read(addr_dst);

    let dst = naitou_square_from_value(dst_value).expect("move dst should be valid");

    match src_value {
        213 => Move::new_drop(ROOK, dst),
        214 => Move::new_drop(BISHOP, dst),
        215 => Move::new_drop(GOLD, dst),
        216 => Move::new_drop(SILVER, dst),
        217 => Move::new_drop(KNIGHT, dst),
        218 => Move::new_drop(LANCE, dst),
        219 => Move::new_drop(PAWN, dst),
        _ => {
            let src = naitou_square_from_value(src_value).expect("move src should be valid");
            let promo = memory_read(addr_promo) != 0;
            if promo {
                Move::new_walk_promotion(src, dst)
            } else {
                Move::new_walk(src, dst)
            }
        }
    }
}

fn read_move_com_impl(addr_src: u16, addr_dst: u16, addr_promo: u16) -> Move {
    let src_value = memory_read(addr_src);
    let dst_value = memory_read(addr_dst);

    let dst = naitou_square_from_value(dst_value).expect("move dst should be valid");

    match src_value {
        201 => Move::new_drop(PAWN, dst),
        202 => Move::new_drop(LANCE, dst),
        203 => Move::new_drop(KNIGHT, dst),
        204 => Move::new_drop(SILVER, dst),
        205 => Move::new_drop(GOLD, dst),
        206 => Move::new_drop(BISHOP, dst),
        207 => Move::new_drop(ROOK, dst),
        _ => {
            let src = naitou_square_from_value(src_value).expect("move src should be valid");
            let promo = memory_read(addr_promo) != 0;
            if promo {
                Move::new_walk_promotion(src, dst)
            } else {
                Move::new_walk(src, dst)
            }
        }
    }
}

/// 現在の進行度管理用手数を読み取る。
pub fn read_progress_ply() -> u8 {
    memory_read(0x05C1)
}

/// 現在の進行度を読み取る。
pub fn read_progress_level() -> u8 {
    memory_read(0x028E)
}

/// 現在のサブ進行度を読み取る。
pub fn read_progress_level_sub() -> u8 {
    memory_read(0x05C8)
}

/// 現在の戦型を読み取る。
pub fn read_formation() -> Formation {
    let value = memory_read(0x05BE);

    match value {
        0 => Formation::Nakabisha,
        1 => Formation::Sikenbisha,
        3 => Formation::Kakugawari,
        4 => Formation::Sujichigai,
        6 => Formation::HumHishaochi,
        7 => Formation::HumNimaiochi,
        8 => Formation::ComHishaochi,
        9 => Formation::ComNimaiochi,
        99 => Formation::Nothing,
        _ => panic!("invalid formation: {}", value),
    }
}

/// 現在のルート局面の評価を読み取る。
pub fn read_root_evaluation() -> RootEvaluation {
    let adv_price = memory_read(0x0280);
    let disadv_price = memory_read(0x0282);
    let power_hum = memory_read(0x05E7);
    let power_com = memory_read(0x05E4);
    let rbp_com = memory_read(0x05EA);

    let king_sq_hum =
        naitou_square_from_value(memory_read(0x027A)).expect("HUM king square should be valid");
    let king_sq_com =
        naitou_square_from_value(memory_read(0x027B)).expect("COM king square should be valid");
    let king_sq = MyArray1::<Square, Side, 2>::from([king_sq_hum, king_sq_com]);

    RootEvaluation {
        adv_price,
        disadv_price,
        power_hum,
        power_com,
        rbp_com,
        king_sq,
    }
}

/// 現在の末端局面(候補手を指した局面)の評価を読み取る。
pub fn read_leaf_evaluation() -> LeafEvaluation {
    let capture_price = memory_read(0x0278);
    let adv_price = memory_read(0x0272);
    let adv_sq = naitou_square_from_value(memory_read(0x0273));
    let disadv_price = memory_read(0x0274);
    let disadv_sq = naitou_square_from_value(memory_read(0x0275));
    let score_posi = memory_read(0x02A4);
    let score_nega = memory_read(0x05E0);
    let hum_king_threat_around25 = memory_read(0x0299);
    let com_king_safety_around25 = memory_read(0x0295);
    let com_king_threat_around25 = memory_read(0x0296);
    let com_king_threat_around8 = memory_read(0x05EB);
    let com_king_choke_count_around8 = memory_read(0x05E5);
    let src_to_com_king = memory_read(0x0298);
    let dst_to_hum_king = memory_read(0x0294);
    let hum_hanging = memory_read(0x05DF) != 0;
    let com_promo_count = memory_read(0x0293);
    let com_loose_count = memory_read(0x0297);

    LeafEvaluation {
        capture_price,
        adv_price,
        adv_sq,
        disadv_price,
        disadv_sq,
        score_posi,
        score_nega,
        hum_king_threat_around25,
        com_king_safety_around25,
        com_king_threat_around25,
        com_king_threat_around8,
        com_king_choke_count_around8,
        src_to_com_king,
        dst_to_hum_king,
        hum_hanging,
        com_promo_count,
        com_loose_count,
        hum_is_checkmated: false, // 直接読み取ることはできない
        is_suicide: false,        // 原作には存在しない
    }
}

/// 現在の最善手を指した局面の評価を読み取る。
pub fn read_best_evaluation() -> LeafEvaluation {
    let capture_price = memory_read(0x028A);
    let adv_price = memory_read(0x0286);
    let adv_sq = naitou_square_from_value(memory_read(0x0287));
    let disadv_price = memory_read(0x0288);
    let disadv_sq = naitou_square_from_value(memory_read(0x0289));
    let score_posi = memory_read(0x02A6);
    let score_nega = memory_read(0x05E2);
    let hum_king_threat_around25 = memory_read(0x02A0);
    let com_king_safety_around25 = memory_read(0x029C);
    let com_king_threat_around25 = memory_read(0x029D);
    let src_to_com_king = memory_read(0x029F);
    let dst_to_hum_king = memory_read(0x029B);
    let com_promo_count = memory_read(0x029A);
    let com_loose_count = memory_read(0x029E);

    LeafEvaluation {
        capture_price,
        adv_price,
        adv_sq,
        disadv_price,
        disadv_sq,
        score_posi,
        score_nega,
        hum_king_threat_around25,
        com_king_safety_around25,
        com_king_threat_around25,
        com_king_threat_around8: 0,      // 原作には存在しない
        com_king_choke_count_around8: 0, // 原作では使われない
        src_to_com_king,
        dst_to_hum_king,
        hum_hanging: false, // 原作には存在しない
        com_promo_count,
        com_loose_count,
        hum_is_checkmated: false, // 直接読み取ることはできない
        is_suicide: false,        // 原作には存在しない
    }
}

/// HUM 玉の詰み判定を読み取る。
pub fn read_hum_is_checkmated() -> bool {
    memory_read(0x05DD) != 0
}

/// タイトル画面から指定した手合割で対局開始する入力シーケンスを返す。
pub fn inputs_start_game(handicap: Handicap) -> Vec<Buttons> {
    let mut inputs = Vec::<Buttons>::new();

    // 各手合割に対応する Select 入力回数。
    let select_count = match handicap {
        Handicap::HumSenteSikenbisha => 0,
        Handicap::HumSenteNakabisha => 1,
        Handicap::HumHishaochi => 2,
        Handicap::HumNimaiochi => 4,
        Handicap::ComSenteSikenbisha => 6,
        Handicap::ComSenteNakabisha => 7,
        Handicap::ComHishaochi => 8,
        Handicap::ComNimaiochi => 10,
    };

    // 起動直後は入力を受け付けないので、少し余裕を持たせる。
    inputs.extend([Buttons::empty(); 10]);

    // 規定回数 Select を入力。
    for _ in 0..select_count {
        inputs.extend([BUTTONS_S, Buttons::empty()]);
    }

    // 対局開始。
    inputs.push(BUTTONS_T);

    inputs
}

/// HUM 側の着手を行う入力シーケンスを返す。
/// 最速入力ではなく、ある程度余裕を持たせてある。
///
/// 着手後 20 フレームほど演出が入るので、この関数から戻るまでに思考ルーチンが実行されることはない。
pub fn inputs_move(mv: Move) -> Vec<Buttons> {
    // カーソル移動の入力シーケンスを返す。
    fn cursor_motion(src: Cursor, dst: Cursor, interval: usize) -> Vec<Buttons> {
        let mut inputs = Vec::<Buttons>::new();

        for &buttons in apsp::shortest_path(src, dst) {
            inputs.push(buttons);
            inputs.extend(std::iter::repeat(Buttons::empty()).take(interval));
        }

        inputs
    }

    let mut inputs = Vec::<Buttons>::new();

    // 指し手をカーソル対に変換。
    let (cursor_src, cursor_dst) = move_to_cursors(mv);

    // 1F の無入力を入れないとごく稀に再現失敗する。
    inputs.push(Buttons::empty());

    // 現在のカーソル位置から cursor_src へ移動する入力。
    inputs.extend(cursor_motion(read_cursor(), cursor_src, 6));

    // 駒をつかむ入力。
    inputs.push(BUTTONS_A);
    inputs.extend([Buttons::empty(); 5]);

    // cursor_src から cursor_dst へ移動する入力。
    inputs.extend(cursor_motion(cursor_src, cursor_dst, 6));

    // 着手確定の入力。
    inputs.push(BUTTONS_A);

    // 成り/不成の入力。これが必要かどうかは場合によるが、
    // どうせ着手後の演出時間による余裕があるので、常に入力しても問題ない。
    if mv.is_promotion() {
        inputs.extend([Buttons::empty(), BUTTONS_A]);
    } else {
        inputs.extend([Buttons::empty(), BUTTONS_D, Buttons::empty(), BUTTONS_A]);
    }

    inputs
}

/// HUM 側の指し手を (移動元カーソル位置, 移動先カーソル位置) に変換する。
pub fn move_to_cursors(mv: Move) -> (Cursor, Cursor) {
    let cursor_dst = Cursor::new_board(mv.dst());

    let cursor_src = if mv.is_drop() {
        Cursor::new_hand(mv.dropped_piece_kind())
    } else {
        Cursor::new_board(mv.src())
    };

    (cursor_src, cursor_dst)
}
