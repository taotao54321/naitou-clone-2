use crate::shogi::*;

/// (手番, 盤面, 両陣営の手駒, 指し手の配列) を sfen 文字列にエンコードする。
/// 合法性チェックは一切行わない。
///
/// 開始局面が平手初期局面の場合、局面文字列は "startpos" になる。
///
/// 待ったフラグを持つ指し手は先頭に '!' を付けて表す(独自拡張)。
pub fn sfen_encode<T>(side_to_move: Side, board: &Board, hands: &Hands, mvs: T) -> String
where
    T: AsRef<[Move]>,
{
    let mut s = sfen_encode_position(side_to_move, board, hands);

    s.push_str(" moves");

    for &mv in mvs.as_ref() {
        s.push(' ');
        sfen_encode_move_impl(mv, &mut s);
    }

    s
}

/// (手番, 盤面, 両陣営の手駒) を sfen 局面文字列にエンコードする。
/// 合法性チェックは一切行わない。
///
/// 局面が平手初期局面の場合、"startpos" を返す。
pub fn sfen_encode_position(side_to_move: Side, board: &Board, hands: &Hands) -> String {
    if side_to_move == HUM && *board == Board::startpos() && hands.iter().all(Hand::is_empty) {
        return "startpos".to_owned();
    }

    let mut s = String::new();

    s.push_str("sfen ");

    sfen_encode_board(board, &mut s);
    s.push(' ');

    sfen_encode_side(side_to_move, &mut s);
    s.push(' ');

    sfen_encode_hands(hands, &mut s);
    s.push(' ');

    // 手数は 1 固定とする。
    sfen_encode_ply(1, &mut s);

    s
}

/// 盤面を sfen 盤面文字列にエンコードし、既存の文字列に追記する。
/// 合法性チェックは一切行わない。
fn sfen_encode_board(board: &Board, s: &mut String) {
    for row in Row::iter() {
        if row != ROW_1 {
            s.push('/');
        }
        sfen_encode_board_row(board, row, s);
    }
}

fn sfen_encode_board_row(board: &Board, row: Row, s: &mut String) {
    #[derive(Debug)]
    struct State<'a> {
        s: &'a mut String,
        run_blank: u32,
    }
    impl<'a> State<'a> {
        fn new(s: &'a mut String) -> Self {
            Self { s, run_blank: 0 }
        }
        fn update(&mut self, pc: Piece) {
            if pc == NO_PIECE {
                self.run_blank += 1;
            } else {
                self.flush_run();
                sfen_encode_board_piece(pc, self.s);
            }
        }
        fn flush_run(&mut self) {
            if self.run_blank > 0 {
                self.s.push(char::from_digit(self.run_blank, 10).unwrap());
                self.run_blank = 0;
            }
        }
    }

    let mut state = State::new(s);
    for col in Col::iter().rev() {
        let sq = Square::from_col_row(col, row);
        state.update(board[sq]);
    }
    state.flush_run();
}

fn sfen_encode_board_piece(pc: Piece, s: &mut String) {
    let side = pc.side();
    let pk = pc.kind();

    if pk.is_promoted() {
        s.push('+');
    }

    let pr = if pk == KING { KING } else { pk.to_raw() };

    let c = match (side, pr) {
        (HUM, KING) => 'K',
        (HUM, ROOK) => 'R',
        (HUM, BISHOP) => 'B',
        (HUM, GOLD) => 'G',
        (HUM, SILVER) => 'S',
        (HUM, KNIGHT) => 'N',
        (HUM, LANCE) => 'L',
        (HUM, PAWN) => 'P',
        (COM, KING) => 'k',
        (COM, ROOK) => 'r',
        (COM, BISHOP) => 'b',
        (COM, GOLD) => 'g',
        (COM, SILVER) => 's',
        (COM, KNIGHT) => 'n',
        (COM, LANCE) => 'l',
        (COM, PAWN) => 'p',
        _ => panic!("invalid piece: {:?}", pc),
    };
    s.push(c);
}

/// 手番の陣営を sfen 手番文字列にエンコードし、既存の文字列に追記する。
fn sfen_encode_side(side_to_move: Side, s: &mut String) {
    match side_to_move {
        HUM => s.push('b'),
        COM => s.push('w'),
        _ => panic!("invalid side to move: {:?}", side_to_move),
    }
}

/// 両陣営の手駒を sfen 手駒文字列にエンコードし、既存の文字列に追記する。
/// 合法性チェックは一切行わない。
fn sfen_encode_hands(hands: &Hands, s: &mut String) {
    // sfen の仕様では手駒の順番を以下のように規定している:
    //
    // * 全ての先手の手駒、全ての後手の手駒の順に並べる。
    // * 駒種は飛、角、金、銀、桂、香、歩の順に並べる。
    //
    // ref:
    //
    // * https://yaneuraou.yaneu.com/2016/07/15/sfen%e6%96%87%e5%ad%97%e5%88%97%e3%81%af%e6%9c%ac%e6%9d%a5%e3%81%af%e4%b8%80%e6%84%8f%e3%81%ab%e5%ae%9a%e3%81%be%e3%82%8b%e4%bb%b6/
    // * https://web.archive.org/web/20080131070731/http://www.glaurungchess.com/shogi/usi.html

    use std::fmt::Write as _;

    const PKS: [PieceKind; 7] = [ROOK, BISHOP, GOLD, SILVER, KNIGHT, LANCE, PAWN];

    if hands.iter().all(Hand::is_empty) {
        s.push('-');
        return;
    }

    for side in Side::iter() {
        for pk in PKS {
            let n = hands[side][pk];
            if n == 0 {
                continue;
            }

            if n >= 2 {
                write!(s, "{}", n).unwrap();
            }
            sfen_encode_hand_piece(side, pk, s);
        }
    }
}

fn sfen_encode_hand_piece(side: Side, pk: PieceKind, s: &mut String) {
    let c = match (side, pk) {
        (HUM, ROOK) => 'R',
        (HUM, BISHOP) => 'B',
        (HUM, GOLD) => 'G',
        (HUM, SILVER) => 'S',
        (HUM, KNIGHT) => 'N',
        (HUM, LANCE) => 'L',
        (HUM, PAWN) => 'P',
        (COM, ROOK) => 'r',
        (COM, BISHOP) => 'b',
        (COM, GOLD) => 'g',
        (COM, SILVER) => 's',
        (COM, KNIGHT) => 'n',
        (COM, LANCE) => 'l',
        (COM, PAWN) => 'p',
        _ => panic!("invalid hand piece: side={:?}, kind={:?}", side, pk),
    };
    s.push(c);
}

/// 手数を sfen 手数文字列にエンコードし、既存の文字列に追記する。
fn sfen_encode_ply(ply: u32, s: &mut String) {
    use std::fmt::Write as _;

    write!(s, "{}", ply).unwrap();
}

/// 指し手を sfen 指し手文字列にエンコードする。
/// 合法性チェックは一切行わない。
pub fn sfen_encode_move(mv: Move) -> String {
    let mut s = String::new();

    sfen_encode_move_impl(mv, &mut s);

    s
}

/// 指し手を sfen 指し手文字列にエンコードし、既存の文字列に追記する。
/// 合法性チェックは一切行わない。
fn sfen_encode_move_impl(mv: Move, s: &mut String) {
    // 待ったフラグを持つ指し手は先頭に '!' を付けて表す(独自拡張)。
    if mv.is_matta() {
        s.push('!');
    }

    if mv.is_drop() {
        sfen_encode_move_drop(mv, s);
    } else {
        sfen_encode_move_walk(mv, s);
    }
}

fn sfen_encode_move_walk(mv_walk: Move, s: &mut String) {
    sfen_encode_move_col(mv_walk.src().col(), s);
    sfen_encode_move_row(mv_walk.src().row(), s);
    sfen_encode_move_col(mv_walk.dst().col(), s);
    sfen_encode_move_row(mv_walk.dst().row(), s);

    if mv_walk.is_promotion() {
        s.push('+');
    }
}

fn sfen_encode_move_drop(mv_drop: Move, s: &mut String) {
    sfen_encode_move_drop_piece_kind(mv_drop.dropped_piece_kind(), s);

    s.push('*');

    sfen_encode_move_col(mv_drop.dst().col(), s);
    sfen_encode_move_row(mv_drop.dst().row(), s);
}

fn sfen_encode_move_col(col: Col, s: &mut String) {
    let d = u32::try_from(col - COL_1 + 1).unwrap();
    let c = char::from_digit(d, 10).unwrap();
    s.push(c);
}

fn sfen_encode_move_row(row: Row, s: &mut String) {
    let n = u8::try_from(row - ROW_1).unwrap();
    let c = char::from(b'a' + n);
    s.push(c);
}

fn sfen_encode_move_drop_piece_kind(pk: PieceKind, s: &mut String) {
    let c = match pk {
        ROOK => 'R',
        BISHOP => 'B',
        GOLD => 'G',
        SILVER => 'S',
        KNIGHT => 'N',
        LANCE => 'L',
        PAWN => 'P',
        _ => panic!("invalid drop piece kind: {:?}", pk),
    };
    s.push(c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matta() {
        assert_eq!(
            sfen_encode_move(Move::new_matta(Move::new_walk(SQ_77, SQ_76))),
            "!7g7f"
        );
    }
}
