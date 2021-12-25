//! 一人プレイ用のシェル。手動で局面を動かしたり、内部情報を表示したり。

use std::ops::ControlFlow;

use anyhow::{bail, ensure, Context as _};

use naitou_clone::*;

fn main() -> anyhow::Result<()> {
    bbs::init();

    let mut shell = Shell::new();

    shell.interact()?;

    Ok(())
}

#[derive(Debug)]
struct Shell {
    pos: Position,
    history: Vec<UndoableMove>,
}

impl Shell {
    fn new() -> Self {
        let pos = startpos();
        let history = Vec::<UndoableMove>::new();

        Self { pos, history }
    }

    fn interact(&mut self) -> anyhow::Result<()> {
        use std::io::Write as _;

        self.print_position();

        loop {
            println!();
            print!("solo shell > ");
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
            "position" => self.do_command_position(args)?,
            "move" => self.do_command_move(args)?,
            "undo" => self.do_command_undo(args)?,
            "print" => self.do_command_print(args)?,
            _ => bail!("unknown command: {}", cmd),
        }

        Ok(ControlFlow::Continue(()))
    }

    fn do_command_position(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let pos_s = args.join(" ");
        let (side_to_move, board, hands) = sfen_decode_position(pos_s)?;
        let pos = Position::new(side_to_move, board, hands);

        self.pos = pos;
        self.history.clear();

        self.print_position();

        Ok(())
    }

    fn do_command_move(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mv_s = args.get(0).context("move is not specified")?;
        let mv = sfen_decode_move(mv_s)?;

        // TODO: とりあえず疑似合法手を全部受け入れる。本来は合法手に限るべき。
        ensure!(generate_moves(&self.pos).contains(&mv), "illegal move");

        let umv = self.pos.do_move(mv);
        self.history.push(umv);

        self.print_position();

        Ok(())
    }

    fn do_command_undo(&mut self, _args: &[&str]) -> anyhow::Result<()> {
        let umv = self.history.pop().context("history is empty")?;

        self.pos.undo_move(umv);

        self.print_position();

        Ok(())
    }

    fn do_command_print(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let obj_s = *args.get(0).context("object name is not specified")?;

        match obj_s {
            "position" => self.print_position(),
            "effect" => {
                println!("{}", self.pos.effect_count_board(HUM));
                print!("{}", self.pos.effect_count_board(COM));
            }
            "attacker" => self.print_attacker(),
            _ => bail!("unknown object name"),
        }

        Ok(())
    }

    fn print_position(&self) {
        print!("{}", self.pos);
    }

    fn print_attacker(&self) {
        for row in Row::iter() {
            for col in Col::iter().rev() {
                let sq = Square::from_col_row(col, row);
                let atk_hum = naitou_attacker(&self.pos, HUM, sq);
                let atk_com = naitou_attacker(&self.pos, COM, sq);

                print!("[");
                if atk_hum == NO_PIECE_KIND {
                    print!("   ");
                } else {
                    print!(" {}", atk_hum);
                }
                if atk_com == NO_PIECE_KIND {
                    print!("   ");
                } else {
                    print!("v{}", atk_com);
                }
                print!("]");
            }
            println!();
        }
    }
}

fn startpos() -> Position {
    let side_to_move = HUM;
    let board = Board::startpos();
    let hands = Hands::from([Hand::empty(); 2]);

    Position::new(side_to_move, board, hands)
}
