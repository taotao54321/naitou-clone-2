//! 与えられた棋譜について最短の全駒勝利手順を求める。
//! 棋譜の最終局面は終局しておらず、HUM 側の手番でなければならない。

use std::num::NonZeroU32;

use anyhow::{bail, ensure};
use arrayvec::ArrayVec;
use structopt::StructOpt;

use naitou_clone::*;

#[derive(Debug, StructOpt)]
struct Opt {
    /// 原作における時間制限設定。平手の戦型選択に影響する。
    #[structopt(long)]
    timelimit: bool,

    /// スレッド数。省略した場合、論理 CPU 数となる。
    #[structopt(long)]
    thread_count: Option<NonZeroU32>,

    /// マルチスレッド探索を開始する深さ。省略すると探索深さの半分(切り上げ)となる。
    #[structopt(long)]
    branch_depth: Option<NonZeroU32>,

    /// 棋譜の sfen 文字列。
    sfen: String,

    /// 棋譜の最終局面からの探索深さ。
    depth: NonZeroU32,
}

fn main() -> anyhow::Result<()> {
    bbs::init();

    let opt = Opt::from_args();

    let thread_count = opt
        .thread_count
        .unwrap_or_else(|| NonZeroU32::new(u32::try_from(num_cpus::get()).unwrap()).unwrap());
    eprintln!("Thread count: {}", thread_count);

    let branch_depth = opt
        .branch_depth
        .unwrap_or_else(|| NonZeroU32::new((opt.depth.get() + 1) / 2).unwrap());
    assert!(branch_depth.get() <= opt.depth.get());
    eprintln!("Branch depth: {}", branch_depth);
    eprintln!("Target depth: {}", opt.depth);

    let (handicap, engine, history) = init(opt.sfen, opt.timelimit)?;

    let handles: Vec<_> = (0..thread_count.get())
        .map(|thread_id| {
            let engine = engine.clone();
            let history = history.clone();
            std::thread::spawn(move || {
                let thread_count = thread_count.get();
                let branch_depth = opt.depth.get() - branch_depth.get();
                let mut solver = Solver {
                    thread_count,
                    thread_id,
                    branch_depth,
                    branch_idx: thread_count - 1, // ノード番号を 0 から始めるため(最初にインクリメントするので)
                    handicap,
                    engine,
                    history,
                };
                solver.solve(opt.depth.get());
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

/// 与えられた棋譜を元に (手合割, 最終局面まで進めた思考エンジン, 初手からの手順) を返す。
fn init(sfen: impl AsRef<str>, timelimit: bool) -> anyhow::Result<(Handicap, Engine, Vec<Move>)> {
    let (side_to_move, board, hands, mvs_vec) = sfen_decode(sfen)?;

    let handicap = Handicap::from_startpos(side_to_move, &board, &hands, timelimit)?;

    let (mut engine, umv_com) = Engine::new(handicap);
    let mut mvs = mvs_vec.as_slice();

    // COM が先に指す手合割の場合、棋譜の初手(必須)が一致するか確認する。
    if let Some(umv_com) = umv_com {
        ensure!(!mvs.is_empty());
        ensure!(mvs[0] == Move::from(umv_com));
        mvs = &mvs[1..];
    }

    // 棋譜の最終局面までトレース。
    while !mvs.is_empty() {
        let resp = engine.do_step(mvs[0])?;

        match resp {
            EngineResponse::Move(r) => {
                // 通常の指し手なら、現時点で mvs は 2 個以上の指し手を含み、
                // 2 個目の指し手は COM の指し手に一致するはず。
                ensure!(mvs.len() >= 2);
                ensure!(mvs[1] == Move::from(r.move_com()));
            }
            _ => bail!("unexpected game end"),
        }

        mvs = &mvs[2..];
    }

    Ok((handicap, engine, mvs_vec))
}

#[derive(Debug)]
struct Solver {
    // 総スレッド数。
    thread_count: u32,

    // このソルバーを実行するスレッドID。0..=thread_count-1 の値をとる。
    thread_id: u32,

    // マルチスレッド探索を開始する **残り深さ**。
    branch_depth: u32,

    // 残り深さ branch_depth における (ノード番号) % thread_count の値。
    // この値が thread_id と等しい場合のみ探索を続ける。
    // これにより、似た局面がスレッド別に振り分けられ、スレッド間のノード数の偏りが抑えられる。
    // ただし、代償として残り深さが branch_depth より大きい部分木は全スレッドで重複して探索することになる。
    //
    // ref: https://twitter.com/toku51n/status/1479463700721733632
    branch_idx: u32,

    handicap: Handicap,
    engine: Engine,
    history: Vec<Move>,
}

impl Solver {
    /// 残り深さ depth まで探索。
    fn solve(&mut self, depth: u32) {
        let com_nonking_count = self.engine.position().com_nonking_count();

        // (残り深さ) < (COM 駒数) なら明らかに解がないので枝刈りできる。
        if depth < com_nonking_count {
            return;
        }

        if depth == self.branch_depth {
            // branch_idx をインクリメント(mod thread_count)。
            self.branch_idx += 1;
            if self.branch_idx == self.thread_count {
                self.branch_idx = 0;
            }
            // スレッドIDが branch_idx と等しい場合のみ探索を続ける。
            if self.thread_id != self.branch_idx {
                return;
            }
        }

        // HUM 側の指し手(待ったフラグ込み)を列挙する。
        // (残り深さ) == (COM 駒数) ならば駒取りの指し手のみを生成。
        // さもなくば通常の指し手を生成。
        let mvs = {
            let pos = self.engine.position();
            if depth == com_nonking_count {
                generate_captures(pos)
            } else if pos.is_checked(HUM) {
                generate_evasions(pos)
            } else {
                generate_moves(pos)
            }
        };
        let mvs = iter_moves(
            mvs,
            self.engine.progress_ply(),
            self.engine.progress_level(),
        );

        // 指し手を順に試す。自殺手の場合はエラーが返されるのでスキップ。
        for mv in mvs {
            if let Ok(resp) = self.engine.do_step(mv) {
                match &resp {
                    EngineResponse::Move(ref resp_move) => {
                        // 通常の指し手が返されたら探索を進める。
                        self.history.push(mv);
                        self.history.push(Move::from(resp_move.move_com()));
                        self.solve(depth - 1);
                        self.history.truncate(self.history.len() - 2); // 2 回 pop するのと同じ
                    }
                    EngineResponse::HumWin(_) => {
                        // HUM 勝ちで、かつ全駒できていれば棋譜を出力。
                        // 残り深さが branch_depth より大きい場合、全スレッドで重複して探索されるので、
                        // スレッドIDが 0 の場合のみ出力する。
                        let ok = self.engine.position().com_nonking_count() == 0;
                        if ok && (depth <= self.branch_depth || self.thread_id == 0) {
                            self.history.push(mv);
                            self.print_solution();
                            self.history.pop();
                        }
                    }
                    // HUM 負けなら何もしない。
                    _ => {}
                }
                self.engine.undo_step(&resp);
            }
        }
    }

    fn print_solution(&self) {
        let (side_to_move, board, hands) = self.handicap.startpos();

        let sfen = sfen_encode(side_to_move, &board, &hands, &self.history);
        println!("{}", sfen);
    }
}

/// 待ったフラグ込みで疑似合法手を列挙する。
fn iter_moves(mvs: MoveArray, progress_ply: u8, progress_level: u8) -> impl Iterator<Item = Move> {
    // プレイヤー先手の場合、初手では待った技が使えないことに注意。
    // (どうやってもCOMが自力で考えた最善手が quiet にしかならないので)
    let gen_matta = progress_ply != 0 && progress_level == 0;

    mvs.into_iter().flat_map(move |mv| {
        let mut ary = ArrayVec::<Move, 2>::new();
        ary.push(mv);
        if gen_matta {
            ary.push(Move::new_matta(mv));
        }
        ary
    })
}
