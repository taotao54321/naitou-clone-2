//! 思考エンジンと対戦するシェル。

use std::ops::ControlFlow;

use anyhow::{bail, ensure, Context as _};
use structopt::StructOpt;

use naitou_clone::*;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, possible_values = &Handicap::variants(), case_insensitive = true, default_value = "HumSenteSikenbisha")]
    handicap: Handicap,
}

fn main() -> anyhow::Result<()> {
    bbs::init();

    let opt = Opt::from_args();

    let mut shell = Shell::new(opt.handicap);

    shell.interact()?;

    Ok(())
}

#[derive(Debug)]
struct Shell {
    engine: Engine,
    history: Vec<EngineResponse>,
}

impl Shell {
    fn new(handicap: Handicap) -> Self {
        let (engine, _) = Engine::new(handicap);
        let history = Vec::<EngineResponse>::new();

        Self { engine, history }
    }

    fn interact(&mut self) -> anyhow::Result<()> {
        use std::io::Write as _;

        self.print_position();

        loop {
            println!();
            print!("play shell > ");
            std::io::stdout().flush()?;

            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;

            let line = line.trim();
            let tokens: Vec<_> = line.split_ascii_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let cmd = tokens[0];
            let args = &tokens[1..];

            match self.do_command(cmd, args) {
                Ok(ControlFlow::Break(_)) => break,
                Err(e) => println!("error: {}", e),
                _ => {}
            }
        }

        Ok(())
    }

    fn do_command(&mut self, cmd: &str, args: &[&str]) -> anyhow::Result<ControlFlow<()>> {
        match cmd {
            "quit" => return Ok(ControlFlow::Break(())),
            "move" => self.do_command_move(args)?,
            "undo" => self.do_command_undo(args)?,
            "print" => self.do_command_print(args)?,
            _ => bail!("unknown command: {}", cmd),
        }

        Ok(ControlFlow::Continue(()))
    }

    fn do_command_move(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mv_s = args.get(0).context("move is not specified")?;
        let mv = sfen_decode_move(mv_s)?;

        // 少なくとも疑似合法手でなければならない。自殺手チェックは do_step() 内で行われる。
        ensure!(
            generate_moves(self.engine.position()).contains(&mv),
            "illegal move"
        );

        let resp = self.engine.do_step(mv)?;

        match resp {
            EngineResponse::HumWin(_) => println!("終局: あなたの勝ち"),
            EngineResponse::HumSuicide(_) => println!("終局: わたしの勝ち (HUM 自殺手)"),
            EngineResponse::ComWin(_) => println!("終局: わたしの勝ち"),
            _ => {}
        }

        self.history.push(resp);

        self.print_position();

        Ok(())
    }

    fn do_command_undo(&mut self, _args: &[&str]) -> anyhow::Result<()> {
        let resp = self.history.pop().context("history is empty")?;

        self.engine.undo_step(&resp);

        self.print_position();

        Ok(())
    }

    fn do_command_print(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let obj_s = *args.get(0).context("object name is not specified")?;

        match obj_s {
            "position" => self.print_position(),
            _ => bail!("unknown object name"),
        }

        Ok(())
    }

    fn print_position(&self) {
        print!("{}", self.engine.position());
    }
}
