//! 与えられた棋譜について思考ログを出力する。
//! 棋譜は途中終局していてもよいが、原則として HUM の指し手と COM の応手は対になっていなければならない。

use anyhow::ensure;
use structopt::StructOpt;

use naitou_clone::*;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    timelimit: bool,

    sfen: String,
}

fn main() -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .chain(std::io::stdout())
        .apply()?;

    bbs::init();

    let opt = Opt::from_args();

    let (engine, mvs) = init(opt.sfen, opt.timelimit)?;

    trace(engine, &mvs)?;

    Ok(())
}

fn init(sfen: impl AsRef<str>, timelimit: bool) -> anyhow::Result<(Engine, Vec<Move>)> {
    let (side_to_move, board, hands, mut mvs) = sfen_decode(sfen)?;

    let handicap = Handicap::from_startpos(side_to_move, &board, &hands, timelimit)?;

    let (engine, umv_com) = Engine::new(handicap);

    // COM が先に指す手合割の場合、棋譜の初手(必須)を取り出し、一致するか確認する。
    if let Some(umv_com) = umv_com {
        ensure!(!mvs.is_empty());
        let mv = mvs.remove(0);
        ensure!(mv == Move::from(umv_com));
    }

    Ok((engine, mvs))
}

fn trace(mut engine: Engine, mut mvs: &[Move]) -> anyhow::Result<()> {
    // 終局までトレース。
    while !mvs.is_empty() {
        let resp = engine.do_step(mvs[0])?;

        match resp {
            EngineResponse::Move(r) => {
                // 通常の指し手なら、現時点で mvs は 2 個以上の指し手を含み、
                // 2 個目の指し手は COM の指し手に一致するはず。
                ensure!(mvs.len() >= 2);
                ensure!(mvs[1] == Move::from(r.move_com()));
            }
            EngineResponse::ComWin(r) => {
                // COM 勝ちの指し手なら、現時点で mvs はちょうど 2 個の指し手を含み、
                // 2 個目の指し手は COM の指し手に一致するはず。
                // そしてこれは終局なので打ち切る。
                ensure!(mvs.len() == 2);
                ensure!(mvs[1] == Move::from(r.move_com()));
                break;
            }
            EngineResponse::HumWin(_) | EngineResponse::HumSuicide(_) => {
                // HUM 勝ち、または HUM 自殺手なら、現時点で mvs はちょうど 1 個の指し手を含むはず。
                // そしてこれは終局なので打ち切る。
                ensure!(mvs.len() == 1);
                break;
            }
        }

        mvs = &mvs[2..];
    }

    Ok(())
}
