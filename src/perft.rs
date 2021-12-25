use crate::movegen::*;
use crate::position::Position;
use crate::shogi::*;

/// perft の末端ノード。`perft()` のコールバックに渡される。
#[derive(Debug)]
pub struct PerftLeafNode<'a> {
    pos: &'a Position,
    umv: Option<UndoableMove>, // 直前の指し手
    checked: bool,             // 手番の側に王手がかかっているか
    checkmated: bool,          // 手番の側がチェックメイトされているか
}

impl<'a> PerftLeafNode<'a> {
    fn new(pos: &'a Position, umv: Option<UndoableMove>, checked: bool, checkmated: bool) -> Self {
        Self {
            pos,
            umv,
            checked,
            checkmated,
        }
    }

    /// 局面への参照を返す。
    pub fn position(&self) -> &Position {
        self.pos
    }

    /// 直前の指し手を返す。
    pub fn previous_move(&self) -> Option<UndoableMove> {
        self.umv
    }

    /// 手番の側に王手がかかっているかどうかを返す。
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// 手番の側がチェックメイトされているかどうかを返す。
    pub fn is_checkmated(&self) -> bool {
        self.checkmated
    }
}

/// 指定した深さの perft を行う。全ての末端ノードについてコールバックが呼ばれる。
///
/// 深さ depth の全ての合法局面が末端ノードとして列挙される。
/// 現状、以下も合法局面に含まれる:
///
/// * 連続王手の千日手
/// * 打ち歩ステイルメイト
///
/// 関数から戻ったとき、`pos` は呼び出し前の局面に戻っている。
pub fn perft<F>(pos: &mut Position, depth: u32, mut f: F)
where
    F: FnMut(&PerftLeafNode),
{
    perft_dfs(pos, None, depth, &mut f);
}

/// perft 再帰関数。
///
/// 呼び出された時点で `pos` は合法とは限らない。具体的には以下の可能性がある:
///
/// * 手番でない側に王手がかかっている(直前の指し手がある場合、それは自殺手となる)
/// * 直前の指し手が打ち歩詰め
fn perft_dfs<F>(pos: &mut Position, umv: Option<UndoableMove>, depth: u32, f: &mut F)
where
    F: FnMut(&PerftLeafNode),
{
    let us = pos.side_to_move();
    let them = us.inv();

    // 手番でない側に王手がかかっていればこの局面は違法。
    if pos.is_checked(them) {
        return;
    }

    // 手番の側に王手がかかっているか?
    let checked = pos.is_checked(us);

    // 末端ノードでない場合、単に全ての疑似合法手で局面を進める。
    // 手番の側に王手がかかっていれば王手回避手生成、さもなくば通常の疑似合法手生成を行う。
    // いずれの場合も打ち歩詰めが含まれうる。後者の場合は自殺手も含まれうる。
    // これらの違法手は次の再帰呼び出しで検査される。
    // (非末端ノードでは打ち歩詰めは通常の詰みと同様に無視される)
    if depth > 0 {
        let mvs = if checked {
            generate_evasions(pos)
        } else {
            generate_moves(pos)
        };
        for mv in mvs {
            let umv_nxt = pos.do_move(mv);
            perft_dfs(pos, Some(umv_nxt), depth - 1, f);
            pos.undo_move(umv_nxt);
        }
        return;
    }

    // 末端ノードの場合、王手判定、チェックメイト判定を行い、
    // 打ち歩チェックメイトでなければコールバックを呼ぶ。

    // 王手ならばチェックメイト(打ち歩含む)判定。
    let checkmated = checked && pos.is_checkmated();

    // チェックメイトの場合、直前の指し手が歩打ちならば打ち歩チェックメイトなので違法。
    if checkmated {
        if let Some(umv) = umv {
            if umv.is_drop() && umv.dropped_piece_kind() == PAWN {
                return;
            }
        }
    }

    // 現局面は合法なので、コールバックを呼ぶ。
    let leaf = PerftLeafNode::new(pos, umv, checked, checkmated);
    f(&leaf);
}
