//! 全てのカーソル位置のペアについての最短経路。

use arrayvec::ArrayVec;
use once_cell::sync::Lazy;

use crate::shogi::*;
use crate::util;

use super::backend::{
    Buttons, BUTTONS_D, BUTTONS_DL, BUTTONS_DR, BUTTONS_L, BUTTONS_R, BUTTONS_U, BUTTONS_UL,
    BUTTONS_UR,
};
use super::naitou::Cursor;

/// `cursor_src` から `cursor_dst` への最短経路を返す。
pub fn shortest_path(cursor_src: Cursor, cursor_dst: Cursor) -> &'static [Buttons] {
    static APSP: Lazy<Apsp> = Lazy::new(Apsp::new);

    APSP.query(cursor_src, cursor_dst)
}

/// カーソル位置のペアに対する最短経路。
/// どのペアも高々 11 回で互いに到達可能。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ApspEntry(ArrayVec<Buttons, 11>);

impl ApspEntry {
    const fn new() -> Self {
        Self(ArrayVec::<Buttons, 11>::new_const())
    }

    fn as_slice(&self) -> &[Buttons] {
        &self.0
    }
}

/// 全てのカーソル位置のペアに対する最短経路キャッシュ。
/// カーソル位置は盤上 81 マス、HUM 側の手駒 7 種の計 88 通り。
#[derive(Debug)]
struct Apsp([[ApspEntry; 88]; 88]);

impl Apsp {
    /// 全てのカーソル位置のペアに対する最短経路キャッシュを構築する。
    fn new() -> Self {
        const VERTEX_INVALID: usize = 999; // 無効な頂点。
        const DIST_INF: u8 = 100; // 距離無限大を表す値。

        let g = Self::graph();

        // i から j へ最短経路で行くときの (次に辿るべき頂点, 操作)。
        // i == j なら (i, Buttons::empty())
        // 到達不能なら (VERTEX_INVALID, Buttons::empty())
        let mut nxt = [[(VERTEX_INVALID, Buttons::empty()); 88]; 88];
        for i in 0..88 {
            for j in 0..88 {
                if i == j {
                    nxt[i][j] = (i, Buttons::empty());
                    continue;
                }

                let buttons = g[i][j];
                if !buttons.is_empty() {
                    nxt[i][j] = (j, buttons);
                }
            }
        }

        // i から j への最短距離。
        let mut dist = [[DIST_INF; 88]; 88];
        for i in 0..88 {
            for j in 0..88 {
                if i == j {
                    dist[i][j] = 0;
                    continue;
                }

                if !g[i][j].is_empty() {
                    dist[i][j] = 1;
                }
            }
        }

        // Floyd-Warshall algorithm により全点対間最短距離を求める。
        for k in 0..88 {
            for i in 0..88 {
                if dist[i][k] == DIST_INF {
                    continue;
                }
                for j in 0..88 {
                    if dist[k][j] == DIST_INF {
                        continue;
                    }
                    let d_new = dist[i][k] + dist[k][j];
                    if util::chmin(&mut dist[i][j], d_new) {
                        nxt[i][j] = nxt[i][k];
                    }
                }
            }
        }

        // nxt, dist から全点対間最短経路を復元する。

        // ArrayVec は Copy でないため、2 次元配列リテラルでは初期化できない。
        // そこで array::map を経由する。内周の配列は要素を定数にすることでリテラル初期化ができる。
        let mut apsp: [[ApspEntry; 88]; 88] = [(); 88].map(|_| {
            const ELEM: ApspEntry = ApspEntry::new();
            [ELEM; 88]
        });
        for i in 0..88 {
            for j in 0..88 {
                // 任意の 2 点間に経路が存在するはず。
                assert_ne!(dist[i][j], DIST_INF);

                let entry = &mut apsp[i][j];
                let mut v = i;
                while v != j {
                    let (nxt_v, nxt_buttons) = nxt[v][j];
                    entry.0.push(nxt_buttons);
                    v = nxt_v;
                }
            }
        }

        Self(apsp)
    }

    /// `cursor_src` から `cursor_dst` への最短経路を返す。
    fn query(&self, cursor_src: Cursor, cursor_dst: Cursor) -> &[Buttons] {
        let i = Self::vertex(cursor_src);
        let j = Self::vertex(cursor_dst);
        self.0[i][j].as_slice()
    }

    /// カーソル位置をグラフの頂点と見たときの隣接行列を返す。
    ///
    /// 戻り値の (i, j) 要素は、i, j が相異なる隣接する頂点であれば i から j へ移動するための操作、
    /// さもなくば `Buttons::empty()` である。
    fn graph() -> [[Buttons; 88]; 88] {
        // マスの周囲 8 マスについての (筋の差分, 段の差分, 操作)
        const SQUARE_NEIGHBORS: [(i32, i32, Buttons); 8] = [
            (-1, -1, BUTTONS_UR),
            (-1, 0, BUTTONS_R),
            (-1, 1, BUTTONS_DR),
            (0, -1, BUTTONS_U),
            (0, 1, BUTTONS_D),
            (1, -1, BUTTONS_UL),
            (1, 0, BUTTONS_L),
            (1, 1, BUTTONS_DL),
        ];

        let mut g = [[Buttons::empty(); 88]; 88];

        // 盤上のマス間の接続。
        for col_src in Col::iter() {
            for row_src in Row::iter() {
                let sq_src = Square::from_col_row(col_src, row_src);

                for (dcol, drow, buttons) in SQUARE_NEIGHBORS {
                    let col_dst = col_src + dcol;
                    let row_dst = row_src + drow;

                    if col_dst.is_on_board() && row_dst.is_on_board() {
                        let sq_dst = Square::from_col_row(col_dst, row_dst);

                        let i = Self::vertex_square(sq_src);
                        let j = Self::vertex_square(sq_dst);
                        g[i][j] = buttons;
                    }
                }
            }
        }

        // 盤面から手駒への接続。
        for row in Row::iter() {
            let sq = Square::from_col_row(COL_1, row);

            let i = Self::vertex_square(sq);
            g[i][Self::vertex_hand(ROOK)] = BUTTONS_R;
            g[i][Self::vertex_hand(SILVER)] = BUTTONS_DR;
        }

        // 手駒から盤面への接続。
        {
            let i = Self::vertex_hand(ROOK);
            g[i][Self::vertex_square(SQ_16)] = BUTTONS_UL;
            g[i][Self::vertex_square(SQ_17)] = BUTTONS_L;
            g[i][Self::vertex_square(SQ_18)] = BUTTONS_DL;
        }
        {
            let i = Self::vertex_hand(SILVER);
            g[i][Self::vertex_square(SQ_17)] = BUTTONS_UL;
            g[i][Self::vertex_square(SQ_18)] = BUTTONS_L;
            g[i][Self::vertex_square(SQ_19)] = BUTTONS_DL;
        }
        {
            let i = Self::vertex_hand(PAWN);
            g[i][Self::vertex_square(SQ_18)] = BUTTONS_UL;
            g[i][Self::vertex_square(SQ_19)] = BUTTONS_L;
        }

        // 手駒の駒種間の接続。
        {
            let mut add_edge = |pk_src: PieceKind, pk_dst: PieceKind, buttons: Buttons| {
                let i = Self::vertex_hand(pk_src);
                let j = Self::vertex_hand(pk_dst);
                g[i][j] = buttons;
            };

            add_edge(ROOK, BISHOP, BUTTONS_R);
            add_edge(ROOK, SILVER, BUTTONS_D);
            add_edge(ROOK, KNIGHT, BUTTONS_DR);

            add_edge(BISHOP, ROOK, BUTTONS_L);
            add_edge(BISHOP, GOLD, BUTTONS_R);
            add_edge(BISHOP, SILVER, BUTTONS_DL);
            add_edge(BISHOP, KNIGHT, BUTTONS_D);
            add_edge(BISHOP, LANCE, BUTTONS_DR);

            add_edge(GOLD, BISHOP, BUTTONS_L);
            add_edge(GOLD, SILVER, BUTTONS_R);
            add_edge(GOLD, KNIGHT, BUTTONS_DL);
            add_edge(GOLD, LANCE, BUTTONS_D);

            add_edge(SILVER, ROOK, BUTTONS_U);
            add_edge(SILVER, BISHOP, BUTTONS_UR);
            add_edge(SILVER, KNIGHT, BUTTONS_R);
            add_edge(SILVER, PAWN, BUTTONS_D);

            add_edge(KNIGHT, ROOK, BUTTONS_UL);
            add_edge(KNIGHT, BISHOP, BUTTONS_U);
            add_edge(KNIGHT, GOLD, BUTTONS_UR);
            add_edge(KNIGHT, SILVER, BUTTONS_L);
            add_edge(KNIGHT, LANCE, BUTTONS_R);
            add_edge(KNIGHT, PAWN, BUTTONS_DL);

            add_edge(LANCE, BISHOP, BUTTONS_UL);
            add_edge(LANCE, GOLD, BUTTONS_U);
            add_edge(LANCE, KNIGHT, BUTTONS_L);
            add_edge(LANCE, PAWN, BUTTONS_R);

            add_edge(PAWN, SILVER, BUTTONS_U);
        }

        g
    }

    /// カーソル位置を頂点インデックスに変換する。
    fn vertex(cursor: Cursor) -> usize {
        match cursor {
            Cursor::Board(sq) => Self::vertex_square(sq),
            Cursor::Hand(pk) => Self::vertex_hand(pk),
        }
    }

    /// マスを頂点インデックスに変換する。
    fn vertex_square(sq: Square) -> usize {
        usize::from(sq)
    }

    /// HUM 側の手駒の駒種を頂点インデックスに変換する。
    fn vertex_hand(pk: PieceKind) -> usize {
        81 + usize::from(pk) - 1
    }
}
