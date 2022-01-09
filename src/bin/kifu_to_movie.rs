//! 与えられた棋譜を最速で入力する .fm2 ムービーを出力する。

use std::path::PathBuf;

use anyhow::ensure;
use structopt::StructOpt;
use uuid::Uuid;

use naitou_clone::emu::{
    addrs, apsp, Buttons, BUTTONS_A, BUTTONS_B, BUTTONS_D, BUTTONS_L, BUTTONS_R, BUTTONS_S,
    BUTTONS_T, BUTTONS_U,
};
use naitou_clone::*;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    timelimit: bool,

    #[structopt(parse(from_os_str))]
    path_rom: PathBuf,

    sfen: String,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    emu::init(opt.path_rom)?;

    let (handicap, mvs) = parse_sfen(opt.sfen, opt.timelimit)?;

    kifu_to_movie(handicap, &mvs)?;

    Ok(())
}

fn parse_sfen(sfen: impl AsRef<str>, timelimit: bool) -> anyhow::Result<(Handicap, Vec<Move>)> {
    let (side_to_move, board, hands, mvs) = sfen_decode(sfen)?;

    let handicap = Handicap::from_startpos(side_to_move, &board, &hands, timelimit)?;

    Ok((handicap, mvs))
}

fn kifu_to_movie(handicap: Handicap, mut mvs: &[Move]) -> anyhow::Result<()> {
    // FM2 ヘッダを出力。
    print_movie_header();

    // ゲーム開始時の無入力。
    for _ in 0..9 {
        commit_frame(Buttons::empty());
    }

    // 手合割/制限時間設定の入力。
    commit_start_game(handicap);

    // HUM 側の指し手が入力可能になるまで待つ。
    // COM が先に指す手合割の場合、指し手の検証も行う。
    commit_wait_hum_turn();
    if handicap.side_to_move() == COM {
        ensure!(!mvs.is_empty());
        let mv = mvs[0];
        mvs = &mvs[1..];
        ensure!(mv == emu::read_move_com());
    }

    // 終局までトレース。
    while !mvs.is_empty() {
        // HUM 側の指し手を入力。
        // ここで指し手が終わっているなら打ち切る。
        commit_hum_move(mvs[0]);
        if mvs.len() == 1 {
            break;
        }

        // HUM 側の指し手が入力可能になるまで待ち、COM の指し手を検証。
        commit_wait_hum_turn();
        ensure!(mvs[1] == emu::read_move_com());

        mvs = &mvs[2..];
    }

    Ok(())
}

fn commit_start_game(handicap: Handicap) {
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

    for _ in 0..select_count {
        commit_frame(BUTTONS_S);
        commit_frame(Buttons::empty());
    }

    commit_frame(BUTTONS_T);
}

fn commit_wait_hum_turn() {
    let snap = emu::snapshot_create();
    emu::snapshot_save(&snap);

    let mut n_frame = 0;
    let mut done = false;
    while !done {
        n_frame += 1;
        emu::run_frame_hooked_headless(Buttons::empty(), &|addr| {
            if addr == addrs::HUM_TURN {
                done = true;
            }
        });
    }
    assert!(n_frame > 0);

    emu::snapshot_load(&snap);

    // ほとんどの場合、addrs::HUM_TURN が実行された 1F 前から入力可能。
    for _ in 0..n_frame - 1 {
        commit_frame(Buttons::empty());
    }
}

fn commit_hum_move(mv: Move) {
    // 成り選択があるかどうか調べる。
    let promotable = if mv.is_drop() {
        false
    } else {
        let pc = emu::read_board_a()[mv.src()];
        assert!(pc.is_piece());
        assert_eq!(pc.side(), HUM);
        pc.is_promotable() && (mv.src().is_promotion_zone(HUM) || mv.dst().is_promotion_zone(HUM))
    };

    // 移動元と移動先のカーソル位置を得る。
    let (cursor_src, cursor_dst) = emu::move_to_cursors(mv);

    // 現在のカーソル位置から移動元のカーソル位置まで移動。
    {
        let path = apsp::shortest_path(emu::read_cursor(), cursor_src);
        for &buttons in path {
            let cursor = emu::read_cursor();
            commit_fastest_input(buttons, || {
                // 実際にカーソルが動いたか?
                emu::read_cursor() != cursor
            });
        }
        assert_eq!(emu::read_cursor(), cursor_src);
    }

    // 駒をつかむ。
    commit_fastest_input(BUTTONS_A, || {
        // 駒をつかんでいるか?
        for _ in 0..5 {
            emu::run_frame_headless(Buttons::empty());
        }
        emu::memory_read(0xDF) != 0
    });

    // 移動元のカーソル位置から移動先のカーソル位置まで移動。
    {
        let path = apsp::shortest_path(cursor_src, cursor_dst);
        for &buttons in path {
            let cursor = emu::read_cursor();
            commit_fastest_input(buttons, || {
                // 実際にカーソルが動いたか?
                emu::read_cursor() != cursor
            });
        }
        assert_eq!(emu::read_cursor(), cursor_dst);
    }

    // 着手する。
    {
        let snap = emu::snapshot_create();
        emu::snapshot_save(&snap);

        for n_empty in 0.. {
            for _ in 0..n_empty {
                emu::run_frame_headless(Buttons::empty());
            }
            let mut ok = false;
            emu::run_frame_hooked_headless(BUTTONS_A, &|addr| {
                if addr == 0xCE0A {
                    ok = true;
                }
            });

            emu::snapshot_load(&snap);

            if ok {
                for _ in 0..n_empty {
                    commit_frame(Buttons::empty());
                }
                commit_frame(BUTTONS_A);
                break;
            }
        }
    }

    // 成り選択があれば入力する。
    if promotable {
        if mv.is_promotion() {
            commit_frame(Buttons::empty());
            commit_frame(BUTTONS_A);
        } else {
            commit_frame(Buttons::empty());
            commit_frame(BUTTONS_D);
            commit_frame(Buttons::empty());
            commit_frame(BUTTONS_A);
        }
    }
}

/// `buttons` を最速で入力する。
///
/// 入力後に `cond` が満たされることが保証される。
fn commit_fastest_input<F>(buttons: Buttons, cond: F)
where
    F: Fn() -> bool,
{
    let snap = emu::snapshot_create();
    emu::snapshot_save(&snap);

    // 必要な無入力の最小個数を求め、その入力を行う。
    for n_empty in 0.. {
        for _ in 0..n_empty {
            emu::run_frame_headless(Buttons::empty());
        }
        emu::run_frame_headless(buttons);

        let ok = cond();
        emu::snapshot_load(&snap);

        if ok {
            for _ in 0..n_empty {
                commit_frame(Buttons::empty());
            }
            commit_frame(buttons);
            break;
        }
    }
}

fn commit_frame(buttons: Buttons) {
    println!("{}", ButtonsDisplay::from(buttons));

    emu::run_frame_headless(buttons);
}

fn print_movie_header() {
    let guid = Uuid::new_v4();

    println!("version 3");
    println!("emuVersion 20500");
    println!("fourscore 0"); // これを省くと正しく再生されない
    println!("port0 1");
    println!("port1 0");
    println!("port2 0");
    println!("romFilename naitou");
    println!("romChecksum base64:TiPXZM5PONH5qehaPthrKQ==");
    println!(
        "guid {}",
        guid.to_hyphenated()
            .encode_upper(&mut Uuid::encode_buffer())
    );
}

#[derive(Debug)]
struct ButtonsDisplay(Buttons);

impl From<Buttons> for ButtonsDisplay {
    fn from(buttons: Buttons) -> Self {
        Self(buttons)
    }
}

impl std::fmt::Display for ButtonsDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Write as _;

        let button_char = |mask, c| {
            if (self.0 & mask).is_empty() {
                '.'
            } else {
                c
            }
        };

        f.write_str("|0|")?;

        f.write_char(button_char(BUTTONS_R, 'R'))?;
        f.write_char(button_char(BUTTONS_L, 'L'))?;
        f.write_char(button_char(BUTTONS_D, 'D'))?;
        f.write_char(button_char(BUTTONS_U, 'U'))?;
        f.write_char(button_char(BUTTONS_T, 'T'))?;
        f.write_char(button_char(BUTTONS_S, 'S'))?;
        f.write_char(button_char(BUTTONS_B, 'B'))?;
        f.write_char(button_char(BUTTONS_A, 'A'))?;

        f.write_str("|||")?;

        Ok(())
    }
}
