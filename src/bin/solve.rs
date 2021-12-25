//! 与えられた棋譜について最短勝利手順を求める。
//! 棋譜の最終局面は終局しておらず、HUM 側の手番でなければならない。

use std::num::NonZeroU32;

use anyhow::{bail, ensure};
use rayon::prelude::*;
use structopt::StructOpt;

use naitou_clone::*;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    timelimit: bool,

    sfen: String,

    depth: NonZeroU32,
}

fn main() -> anyhow::Result<()> {
    bbs::init();

    let opt = Opt::from_args();

    let (handicap, engine, history) = init(opt.sfen, opt.timelimit)?;

    // HUM 側の初手(疑似合法手)を列挙する。
    let mvs = {
        let pos = engine.position();
        if pos.is_checked(HUM) {
            generate_evasions(pos)
        } else {
            generate_moves(pos)
        }
    };

    // 全ての初手を複数スレッドで分散処理する。
    mvs.into_par_iter().for_each(|&mv| {
        let mut solver = Solver::new(handicap, engine.clone(), history.clone());
        solver.solve(mv, opt.depth.get());
    });

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
    handicap: Handicap,
    engine: Engine,
    history: Vec<Move>,
}

impl Solver {
    fn new(handicap: Handicap, engine: Engine, history: Vec<Move>) -> Self {
        Self {
            handicap,
            engine,
            history,
        }
    }

    /// 指定した初手で指定した深さまで探索する。
    fn solve(&mut self, mv_first: Move, depth: u32) {
        // 初手を指す。これが自殺手ならエラーが返されるので何もしない。
        if let Ok(resp) = self.engine.do_step(mv_first) {
            self.history.push(mv_first);

            match resp {
                EngineResponse::Move(resp_move) => {
                    // 通常の指し手が返されたら手順を記録し、深さ depth-1 で探索。
                    self.history.push(Move::from(resp_move.move_com()));
                    self.solve_dfs(depth - 1);
                }
                EngineResponse::HumWin(_) => {
                    // HUM 勝ちなら単に棋譜を出力。
                    self.print_solution();
                }
                // HUM 負けなら何もしない。
                _ => {}
            }
        }
    }

    fn solve_dfs(&mut self, depth: u32) {
        if depth == 0 {
            return;
        }

        // HUM 側の疑似合法手を列挙する。
        let mvs = {
            let pos = self.engine.position();
            if pos.is_checked(HUM) {
                generate_evasions(pos)
            } else {
                generate_moves(pos)
            }
        };

        // 疑似合法手を順に試す。自殺手の場合はエラーが返されるのでスキップ。
        for mv in mvs {
            if let Ok(resp) = self.engine.do_step(mv) {
                match &resp {
                    EngineResponse::Move(ref resp_move) => {
                        // 通常の指し手が返されたら探索を進める。
                        self.history.push(mv);
                        self.history.push(Move::from(resp_move.move_com()));
                        self.solve_dfs(depth - 1);
                        self.history.truncate(self.history.len() - 2);
                    }
                    EngineResponse::HumWin(_) => {
                        // HUM 勝ちなら棋譜を出力。
                        self.history.push(mv);
                        self.print_solution();
                        self.history.pop();
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
