use anyhow::{anyhow, bail, ensure, Context as _};

use crate::shogi::*;

/// sfen 文字列をデコードし、(手番, 盤面, 両陣営の手駒, 指し手の配列) を返す。
/// 構文はチェックするが、合法性チェックは一切行わない。
///
/// 文字列の先頭と末尾の空白は無視される。
/// また、最初のトークンが "position" の場合、それは単に無視される。
///
/// 待ったフラグを持つ指し手は先頭に '!' を付けて表す(独自拡張)。
pub fn sfen_decode(s: impl AsRef<str>) -> anyhow::Result<(Side, Board, Hands, Vec<Move>)> {
    // 先頭と末尾の空白は無視する。
    let s = s.as_ref().trim();

    let mut tokens = s.split_ascii_whitespace();

    let (side_to_move, board, hands) = sfen_decode_position_from_iter(&mut tokens)?;

    let mvs = if let Some(moves_magic) = tokens.next() {
        ensure!(
            moves_magic == "moves",
            r#""moves" expected, but got {}"#,
            moves_magic
        );
        tokens
            .map(sfen_decode_move_impl)
            .collect::<Result<_, _>>()?
    } else {
        vec![]
    };

    Ok((side_to_move, board, hands, mvs))
}

/// sfen 局面文字列をデコードし、(手番, 盤面, 両陣営の手駒) を返す。
/// 構文はチェックするが、合法性チェックは一切行わない。
///
/// 文字列の先頭と末尾の空白は無視される。
/// また、最初のトークンが "position" の場合、それは単に無視される。
pub fn sfen_decode_position(s: impl AsRef<str>) -> anyhow::Result<(Side, Board, Hands)> {
    // 先頭と末尾の空白は無視する。
    let s = s.as_ref().trim();

    let mut tokens = s.split_ascii_whitespace();

    let (side_to_move, board, hands) = sfen_decode_position_from_iter(&mut tokens)?;

    if let Some(token) = tokens.next() {
        bail!("position string has redundant token: {}", token);
    }

    Ok((side_to_move, board, hands))
}

fn sfen_decode_position_from_iter<'a, I>(it: &mut I) -> anyhow::Result<(Side, Board, Hands)>
where
    I: Iterator<Item = &'a str>,
{
    let mut it = it.peekable();

    // 最初のトークンが "position" なら単に無視する。
    // 外部アプリは "position" を付けたり付けなかったりまちまちなので、それへの対処。
    if it.peek().context("position string is empty")? == &"position" {
        it.next();
    }

    let magic = it.next().context("position string is empty")?;

    if magic == "startpos" {
        return Ok((
            HUM,
            Board::startpos(),
            Hands::from([Hand::empty(), Hand::empty()]),
        ));
    }

    ensure!(magic == "sfen", "invalid position string magic: {}", magic);

    let board = sfen_decode_board(it.next().context("board string not found")?)?;
    let side_to_move = sfen_decode_side(it.next().context("side string not found")?)?;
    let hands = sfen_decode_hands(it.next().context("hands string not found")?)?;
    let _ = sfen_decode_ply(it.next().context("ply string not found")?)?;

    Ok((side_to_move, board, hands))
}

/// sfen 盤面文字列をデコードし、その盤面を返す。合法性チェックは一切行わない。
fn sfen_decode_board(s: &str) -> anyhow::Result<Board> {
    let mut board = Board::empty();

    let mut it = s.split('/');

    for row in Row::iter() {
        let row_s = it.next().context("board string must have exactly 9 rows")?;
        sfen_decode_board_row(row_s, row, &mut board)?;
    }

    if let Some(s) = it.next() {
        bail!("board string has redundant row: {}", s);
    }

    Ok(board)
}

fn sfen_decode_board_row(s: &str, row: Row, board: &mut Board) -> anyhow::Result<()> {
    #[derive(Debug)]
    struct State<'a> {
        board: &'a mut Board,
        col: Col,
        row: Row,
        promo: bool,
    }
    impl<'a> State<'a> {
        fn new(board: &'a mut Board, row: Row) -> Self {
            Self {
                board,
                row,
                col: COL_9,
                promo: false,
            }
        }
        fn update(&mut self, c: char) -> anyhow::Result<()> {
            match c {
                '+' => {
                    ensure!(!self.promo, "double '+' is not allowed");
                    self.check_row_overflow(1)?;
                    self.promo = true;
                }
                '1'..='9' => {
                    ensure!(!self.promo, "'+' cannot be placed before digit");
                    let n = c.to_digit(10).unwrap() as i32;
                    self.check_row_overflow(n)?;
                    self.col -= n;
                }
                _ => {
                    let (side, mut pk) = sfen_decode_board_piece(c)?;
                    self.check_row_overflow(1)?;
                    if self.promo {
                        ensure!(pk.is_promotable(), "not promotable piece: {}", c);
                        pk = pk.to_promoted();
                        self.promo = false;
                    }
                    let sq = Square::from_col_row(self.col, self.row);
                    self.board[sq] = Piece::new(side, pk);
                    self.col -= 1;
                }
            }
            Ok(())
        }
        fn finalize(&self) -> anyhow::Result<()> {
            ensure!(!self.promo, "remaining promotion flag");
            ensure!(
                self.col + 1 == COL_1,
                "board row must have exactly 9 columns"
            );
            Ok(())
        }
        fn check_row_overflow(&self, col_sub: i32) -> anyhow::Result<()> {
            ensure!(self.col - col_sub + 1 >= COL_1, "row overflow");
            Ok(())
        }
    }

    let mut state = State::new(board, row);
    for c in s.chars() {
        state.update(c)?;
    }
    state.finalize()?;

    Ok(())
}

fn sfen_decode_board_piece(c: char) -> anyhow::Result<(Side, PieceKind)> {
    match c {
        'K' => Ok((HUM, KING)),
        'R' => Ok((HUM, ROOK)),
        'B' => Ok((HUM, BISHOP)),
        'G' => Ok((HUM, GOLD)),
        'S' => Ok((HUM, SILVER)),
        'N' => Ok((HUM, KNIGHT)),
        'L' => Ok((HUM, LANCE)),
        'P' => Ok((HUM, PAWN)),
        'k' => Ok((COM, KING)),
        'r' => Ok((COM, ROOK)),
        'b' => Ok((COM, BISHOP)),
        'g' => Ok((COM, GOLD)),
        's' => Ok((COM, SILVER)),
        'n' => Ok((COM, KNIGHT)),
        'l' => Ok((COM, LANCE)),
        'p' => Ok((COM, PAWN)),
        _ => bail!("invalid board piece char: {}", c),
    }
}

/// sfen 手番文字列をデコードし、手番の陣営を返す。
fn sfen_decode_side(s: &str) -> anyhow::Result<Side> {
    match s {
        "b" => Ok(HUM),
        "w" => Ok(COM),
        _ => bail!("invalid side string: {}", s),
    }
}

/// sfen 手駒文字列をデコードし、両陣営の手駒を返す。合法性チェックは一切行わない。
fn sfen_decode_hands(s: &str) -> anyhow::Result<Hands> {
    if s == "-" {
        return Ok(Hands::from([Hand::empty(); 2]));
    }

    #[derive(Debug)]
    struct State {
        hands: Hands,
        count: u8,
    }
    impl State {
        fn new() -> Self {
            Self {
                hands: Hands::from([Hand::empty(); 2]),
                count: 0,
            }
        }
        fn update(&mut self, c: char) -> anyhow::Result<()> {
            match c {
                '0'..='9' => {
                    ensure!(
                        !(c == '0' && self.count == 0),
                        "leading zero is not allowed"
                    );
                    self.count = self.count.checked_mul(10).context("count is too large")?;
                    let d = c.to_digit(10).unwrap() as u8;
                    self.count = self.count.checked_add(d).context("count is too large")?;
                }
                _ => {
                    let (side, pk) = sfen_decode_hand_piece(c)?;
                    let n = if self.count == 0 { 1 } else { self.count };
                    self.hands[side][pk] = self.hands[side][pk]
                        .checked_add(u32::from(n))
                        .context("hand overflow")?;
                    self.count = 0;
                }
            }
            Ok(())
        }
        fn finalize(&self) -> anyhow::Result<()> {
            ensure!(self.count == 0, "remaining count specifier");
            Ok(())
        }
    }

    let mut state = State::new();
    for c in s.chars() {
        state.update(c)?;
    }
    state.finalize()?;

    Ok(state.hands)
}

fn sfen_decode_hand_piece(c: char) -> anyhow::Result<(Side, PieceKind)> {
    match c {
        'R' => Ok((HUM, ROOK)),
        'B' => Ok((HUM, BISHOP)),
        'G' => Ok((HUM, GOLD)),
        'S' => Ok((HUM, SILVER)),
        'N' => Ok((HUM, KNIGHT)),
        'L' => Ok((HUM, LANCE)),
        'P' => Ok((HUM, PAWN)),
        'r' => Ok((COM, ROOK)),
        'b' => Ok((COM, BISHOP)),
        'g' => Ok((COM, GOLD)),
        's' => Ok((COM, SILVER)),
        'n' => Ok((COM, KNIGHT)),
        'l' => Ok((COM, LANCE)),
        'p' => Ok((COM, PAWN)),
        _ => bail!("invalid hand piece char: {}", c),
    }
}

/// sfen 手数文字列をデコードし、その手数を返す。
fn sfen_decode_ply(s: &str) -> anyhow::Result<u32> {
    let ply: u32 = s.parse()?;
    ensure!(ply >= 1, "ply must be positive");

    Ok(ply)
}

/// sfen 指し手文字列をデコードし、その指し手を返す。
/// 構文はチェックするが、合法性チェックは一切行わない。
///
/// 文字列の先頭と末尾の空白は無視される。
pub fn sfen_decode_move(s: impl AsRef<str>) -> anyhow::Result<Move> {
    // 先頭と末尾の空白は無視する。
    let s = s.as_ref().trim();

    sfen_decode_move_impl(s)
}

fn sfen_decode_move_impl(s: &str) -> anyhow::Result<Move> {
    // 待ったフラグを持つ指し手は先頭に '!' を付けて表す(独自拡張)。
    let c_first = *s
        .chars()
        .peekable()
        .peek()
        .context("move string is empty")?;
    let (s, matta) = if c_first == '!' {
        (&s[1..], true)
    } else {
        (s, false)
    };

    sfen_decode_move_walk(s)
        .or_else(|| sfen_decode_move_drop(s))
        .ok_or_else(|| anyhow!("invalid move string: {}", s))
        .map(|mv| if matta { Move::new_matta(mv) } else { mv })
}

fn sfen_decode_move_walk(s: &str) -> Option<Move> {
    let mut it = s.chars();

    let src_col = sfen_decode_move_col(it.next()?)?;
    let src_row = sfen_decode_move_row(it.next()?)?;
    let dst_col = sfen_decode_move_col(it.next()?)?;
    let dst_row = sfen_decode_move_row(it.next()?)?;

    let promo = if let Some(c) = it.next() {
        (c == '+').then(|| true)?
    } else {
        false
    };

    it.next().is_none().then(|| {
        let src = Square::from_col_row(src_col, src_row);
        let dst = Square::from_col_row(dst_col, dst_row);
        if promo {
            Move::new_walk_promotion(src, dst)
        } else {
            Move::new_walk(src, dst)
        }
    })
}

fn sfen_decode_move_drop(s: &str) -> Option<Move> {
    let mut it = s.chars();

    let pk = sfen_decode_move_drop_piece_kind(it.next()?)?;

    if it.next()? != '*' {
        return None;
    }

    let dst_col = sfen_decode_move_col(it.next()?)?;
    let dst_row = sfen_decode_move_row(it.next()?)?;

    it.next().is_none().then(|| {
        let dst = Square::from_col_row(dst_col, dst_row);
        Move::new_drop(pk, dst)
    })
}

fn sfen_decode_move_col(c: char) -> Option<Col> {
    match c {
        '1'..='9' => {
            let n = c.to_digit(10).unwrap() as i32;
            Some(COL_1 - 1 + n)
        }
        _ => None,
    }
}

fn sfen_decode_move_row(c: char) -> Option<Row> {
    match c {
        'a'..='i' => {
            let n = i32::from(c as u8 - b'a');
            Some(ROW_1 + n)
        }
        _ => None,
    }
}

fn sfen_decode_move_drop_piece_kind(c: char) -> Option<PieceKind> {
    match c {
        'R' => Some(ROOK),
        'B' => Some(BISHOP),
        'G' => Some(GOLD),
        'S' => Some(SILVER),
        'N' => Some(KNIGHT),
        'L' => Some(LANCE),
        'P' => Some(PAWN),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matta() {
        assert_eq!(
            sfen_decode_move("!7g7f").unwrap(),
            Move::new_matta(Move::new_walk(SQ_77, SQ_76))
        );
    }
}
