//! 与えられた棋譜をエミュレータ上で再生しつつ、思考ログを出力する。
//! 棋譜は途中終局していてもよいが、原則として HUM の指し手と COM の応手は対になっていなければならない。

use std::path::PathBuf;

use anyhow::{anyhow, ensure};
use sdl2::{
    event::Event,
    pixels::PixelFormatEnum,
    render::{Texture, WindowCanvas},
    EventPump,
};
use structopt::StructOpt;

use naitou_clone::emu::{addrs, Buttons};
use naitou_clone::mylog::*;
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
    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("{}", message)))
        .chain(std::io::stdout())
        .apply()?;

    bbs::init();

    let opt = Opt::from_args();

    emu::init(opt.path_rom)?;

    let (handicap, mvs) = parse_sfen(opt.sfen, opt.timelimit)?;

    let sdl = sdl2::init().map_err(|e| anyhow!(e))?;
    let sdl_video = sdl.video().map_err(|e| anyhow!(e))?;
    let event_pump = sdl.event_pump().map_err(|e| anyhow!(e))?;

    let window = sdl_video.window("emu_trace", 512, 480).build()?;
    let canvas = window.into_canvas().build()?;
    let tex_creator = canvas.texture_creator();
    let tex = tex_creator.create_texture_streaming(PixelFormatEnum::RGBX8888, 256, 240)?;

    let mut app = App {
        event_pump,
        canvas,
        tex,
    };

    app.run(handicap, &mvs)?;

    Ok(())
}

fn parse_sfen(sfen: impl AsRef<str>, timelimit: bool) -> anyhow::Result<(Handicap, Vec<Move>)> {
    let (side_to_move, board, hands, mvs) = sfen_decode(sfen)?;

    let handicap = Handicap::from_startpos(side_to_move, &board, &hands, timelimit)?;

    Ok((handicap, mvs))
}

struct App<'tex> {
    event_pump: EventPump,
    canvas: WindowCanvas,
    tex: Texture<'tex>,
}

impl<'tex> App<'tex> {
    fn run(&mut self, handicap: Handicap, mut mvs: &[Move]) -> anyhow::Result<()> {
        self.start_game(handicap)?;

        // COM が先に指す手合割の場合、棋譜の初手(必須)を取り出し、一致するか確認する。
        if handicap.side_to_move() == COM {
            self.wait_com_move()?;
            ensure!(!mvs.is_empty());
            let mv = mvs[0];
            mvs = &mvs[1..];
            ensure!(mv == emu::read_move_com());
        }

        // 終局までトレース。
        while !mvs.is_empty() {
            self.wait_hum_turn()?;
            self.do_move_hum(mvs[0])?;

            match self.wait_com_move()? {
                ComResponse::Move(mv) => {
                    // 通常の指し手なら、現時点で mvs は 2 個以上の指し手を含み、
                    // 2 個目の指し手は COM の指し手に一致するはず。
                    ensure!(mvs.len() >= 2);
                    ensure!(mvs[1] == mv);
                }
                ComResponse::ComWin(mv) => {
                    // COM 勝ちの指し手なら、現時点で mvs はちょうど 2 個の指し手を含み、
                    // 2 個目の指し手は COM の指し手に一致するはず。
                    // そしてこれは終局なので打ち切る。
                    ensure!(mvs.len() == 2);
                    ensure!(mvs[1] == mv);
                    break;
                }
                ComResponse::HumWin | ComResponse::HumSuicide => {
                    // HUM 勝ち、または HUM 自殺手なら、現時点で mvs はちょうど 1 個の指し手を含むはず。
                    // そしてこれは終局なので打ち切る。
                    ensure!(mvs.len() == 1);
                    break;
                }
            }

            mvs = &mvs[2..];
        }

        // 終局後もしばらく動作させる。
        for _ in 0..500 {
            self.run_frame(Buttons::empty())?;
        }

        Ok(())
    }

    /// 指定された手合割で対局を開始する。
    fn start_game(&mut self, handicap: Handicap) -> anyhow::Result<()> {
        for buttons in emu::inputs_start_game(handicap) {
            self.run_frame(buttons)?;
        }

        Ok(())
    }

    /// HUM 側の着手を行う。
    /// 呼ばれた時点で HUM 側の指し手入力待ちループに入っていなければならない。
    fn do_move_hum(&mut self, mv: Move) -> anyhow::Result<()> {
        for buttons in emu::inputs_move(mv) {
            self.run_frame(buttons)?;
        }

        Ok(())
    }

    /// HUM 側の指し手入力待ちループに入るまで待つ。
    fn wait_hum_turn(&mut self) -> anyhow::Result<()> {
        let mut done = false;
        while !done {
            self.run_frame_hooked(Buttons::empty(), &|addr| {
                if addr == addrs::HUM_TURN {
                    done = true;
                }
            })?;
        }

        Ok(())
    }

    /// COM 側の着手が決まるまで待つ。HUM 側の指し手入力待ちループまでは待たない。
    fn wait_com_move(&mut self) -> anyhow::Result<ComResponse> {
        let mut resp: Option<ComResponse> = None;
        while resp.is_none() {
            self.run_frame_hooked(Buttons::empty(), &|addr| match addr {
                addrs::THINK_START => {
                    log_think_start(emu::read_ply());

                    // この時点では局面 A と局面 B は等しいので、どちらを選んでもよい。
                    log_position(
                        emu::read_side_to_move(),
                        &emu::read_board_a(),
                        &emu::read_hands_a(),
                    );

                    log_effect_count_board(HUM, &emu::read_effect_count_board(HUM));
                    log_effect_count_board(COM, &emu::read_effect_count_board(COM));

                    log_progress(
                        emu::read_progress_ply(),
                        emu::read_progress_level(),
                        emu::read_progress_level_sub(),
                    );
                    log_formation(emu::read_formation());
                }

                addrs::THINK_EVALUATED_ROOT => {
                    log_root_evaluation(&emu::read_root_evaluation());
                }

                addrs::THINK_CAND_START_WALK | addrs::THINK_CAND_START_DROP => {
                    log_cand_start(emu::read_move_cand());
                    log_board(&emu::read_board_b());
                    log_effect_count_board(HUM, &emu::read_effect_count_board(HUM));
                    log_effect_count_board(COM, &emu::read_effect_count_board(COM));
                }

                addrs::THINK_CAND_REJECT_BY_SACRIFICE => {
                    log_cand_reject_by_sacrifice();
                }
                addrs::THINK_CAND_REJECT_BY_DROP_PAWN_MATE => {
                    log_cand_reject_by_drop_pawn_mate();
                }

                addrs::THINK_CAND_REVISE_CAPTURE_BY_PAWN => {
                    log_revise_capture_by_pawn(&emu::read_leaf_evaluation());
                }

                addrs::THINK_CAND_EVALUATED_INI => {
                    log_leaf_evaluation_ini(&emu::read_leaf_evaluation());
                }

                addrs::THINK_CAND_REVISE_HUM_HANGING => {
                    log_revise_hum_hanging(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_MIDGAME_ATTACKED_PAWN => {
                    // 原作では駒損マスがなくても判定を行うが、この場合 disadv_price が 0 なので実質 NOP。
                    // verify の都合上、このケースではログ出力しない。
                    let leaf_eval = emu::read_leaf_evaluation();
                    if leaf_eval.disadv_sq.is_some() {
                        log_revise_midgame_attacked_pawn(&leaf_eval);
                    }
                }
                addrs::THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_ADV_SQ => {
                    // 原作では駒得マスがなくても判定を行うが、この場合 adv_price が 0 なので実質 NOP。
                    // verify の都合上、このケースではログ出力しない。
                    let leaf_eval = emu::read_leaf_evaluation();
                    if leaf_eval.adv_sq.is_some() {
                        log_revise_endgame_unimportant_adv_sq(&leaf_eval);
                    }
                }
                addrs::THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_CHEAP_DISADV_SQ => {
                    // 原作では駒損マスがなくても判定を行うが、この場合 disadv_price が 0 なので実質 NOP。
                    // verify の都合上、このケースではログ出力しない。
                    let leaf_eval = emu::read_leaf_evaluation();
                    if leaf_eval.disadv_sq.is_some() {
                        log_revise_endgame_unimportant_cheap_disadv_sq(&leaf_eval);
                    }
                }
                addrs::THINK_CAND_REVISE_ENDGAME_CAPTURE_NEAR_HUM_KING => {
                    log_revise_endgame_capture_near_hum_king(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_ENDGAME_UNIMPORTANT_CAPTURE => {
                    log_revise_endgame_unimportant_capture(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_USELESS_CHECK => {
                    log_revise_useless_check(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_USELESS_DROP => {
                    log_revise_useless_drop(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_INCREASE_CAPTURE_PRICE => {
                    log_revise_increase_capture_price(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_GOOD_ROOK_BISHOP_DROP => {
                    log_revise_good_rook_bishop_drop(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_BAD_ROOK_BISHOP_DROP => {
                    log_revise_bad_rook_bishop_drop(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CAPTURE_BY_KING => {
                    log_revise_capture_by_king(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CHEAP_ADV_SQ_NEAR_HUM_KING => {
                    log_revise_cheap_adv_sq_near_hum_king(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_INHIBIT_BISHOP_EXCHANGE => {
                    log_revise_inhibit_bishop_exchange(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_KEEP_ROOK_BISHOP_IN_EMERGENCY => {
                    log_revise_keep_rook_bishop_in_emergency(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CAPTURE_NEAR_HUM_KING => {
                    log_revise_capture_near_hum_king(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CAPTURE_BY_KING_IN_EMERGENCY => {
                    log_revise_capture_by_king_in_emergency(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CAPTURING_CHECK => {
                    log_revise_capturing_check(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CHEAP_CAPTURE_PRICE => {
                    log_revise_cheap_capture_price(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_BAD_ROOK_BISHOP_DROP_2 => {
                    log_revise_bad_rook_bishop_drop_2(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_PROMOTED_WALK => {
                    log_revise_promoted_walk(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_CHECK_WITH_POWER => {
                    log_revise_check_with_power(&emu::read_leaf_evaluation());
                }
                addrs::THINK_CAND_REVISE_GOOD_CAPTURING_CHECK => {
                    log_revise_good_capturing_check(&emu::read_leaf_evaluation());
                }

                addrs::THINK_CAND_REVISED => {
                    log_leaf_evaluation_revised(&emu::read_leaf_evaluation());

                    // 原作では詰み判定が出ても候補手と最善手の比較処理は通常通り行われるが、
                    // verify の都合上、詰み判定が出た際は比較ログを出力しない。
                    if emu::read_hum_is_checkmated() {
                        log_hum_is_checkmated();
                    } else {
                        log_cmp_start();
                    }
                }

                addrs::THINK_CAND_IS_SUICIDE => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_suicide(false);
                    }
                }
                addrs::THINK_CAND_IS_NOT_SUICIDE => {
                    if !emu::read_hum_is_checkmated() && emu::reg_p().carry() {
                        log_cmp_suicide(true);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_nega_worse_capture_price_worse();
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_BETTER => {
                    if !emu::read_hum_is_checkmated() {
                        let improved = !emu::reg_p().carry() || emu::reg_p().zero();
                        log_cmp_nega_worse_capture_price_better(improved);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_1 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_nega_worse_capture_price_equal(false);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_2 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().zero() {
                        log_cmp_nega_worse_capture_price_equal(false);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_3 => {
                    if !emu::read_hum_is_checkmated()
                        && (!emu::reg_p().carry() || emu::reg_p().zero())
                    {
                        log_cmp_nega_worse_capture_price_equal(false);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_WORSE_CAPTURE_PRICE_EQUAL_4 => {
                    if !emu::read_hum_is_checkmated() {
                        let improved = !emu::reg_p().carry();
                        log_cmp_nega_worse_capture_price_equal(improved);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_1 => {
                    if !emu::read_hum_is_checkmated() && emu::reg_p().carry() {
                        log_cmp_nega_better_extreme();
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_2 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_nega_better_capture_price_better();
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_WORSE => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_nega_better_capture_price_worse(false);
                        } else if !emu::reg_p().zero() {
                            log_cmp_nega_better_capture_price_worse(true);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_1 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_nega_better_capture_price_equal(true);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_2 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().zero() {
                        log_cmp_nega_better_capture_price_equal(true);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_3 => {
                    if !emu::read_hum_is_checkmated()
                        && (!emu::reg_p().carry() || emu::reg_p().zero())
                    {
                        log_cmp_nega_better_capture_price_equal(true);
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_BETTER_CAPTURE_PRICE_EQUAL_4 => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_nega_better_capture_price_equal(false);
                        } else if !emu::reg_p().zero() {
                            log_cmp_nega_better_capture_price_equal(true);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_NEGA_EQUAL => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_nega_equal(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_nega_equal(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_COM_PROMO_COUNT => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_com_promo_count(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_com_promo_count(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_SCORE_POSI => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_score_posi(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_score_posi(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_ADV_PRICE => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_adv_price(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_adv_price(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_DROP_1 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_prefer_walk();
                    }
                }
                addrs::THINK_CAND_CMP_DROP_2 => {
                    if !emu::read_hum_is_checkmated() && !emu::reg_p().carry() {
                        log_cmp_drop_prefer_cheap();
                    }
                }
                addrs::THINK_CAND_CMP_WALK_HUM_KING_THREAT_AROUND25 => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_walk_hum_king_threat_around25(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_walk_hum_king_threat_around25(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_WALK_COM_KING_SAFETY_AROUND25 => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_walk_com_king_safety_around25(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_walk_com_king_safety_around25(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_WALK_COM_KING_THREAT_AROUND25 => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_walk_com_king_threat_around25(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_walk_com_king_threat_around25(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_WALK_COM_LOOSE_COUNT => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_walk_com_loose_count(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_walk_com_loose_count(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_WALK_DST_TO_HUM_KING => {
                    if !emu::read_hum_is_checkmated() {
                        if !emu::reg_p().carry() {
                            log_cmp_walk_dst_to_hum_king(true);
                        } else if !emu::reg_p().zero() {
                            log_cmp_walk_dst_to_hum_king(false);
                        }
                    }
                }
                addrs::THINK_CAND_CMP_WALK_SRC_TO_COM_KING => {
                    if !emu::read_hum_is_checkmated() {
                        let improved = !emu::reg_p().carry();
                        log_cmp_walk_src_to_com_king(improved);
                    }
                }

                addrs::THINK_CAND_END_WALK | addrs::THINK_CAND_END_DROP => {
                    log_best(emu::read_move_best(), &emu::read_best_evaluation());
                    log_cand_end();
                }

                addrs::THINK_BOOK_START => {
                    log_book_start();
                }
                addrs::THINK_BOOK_JUDGE_MOVE => {
                    if emu::reg_p().zero() {
                        let mv = emu::read_move_com();
                        log_book_accept_move(mv);
                    }
                }

                addrs::THINK_END_MOVE => {
                    let mv = emu::read_move_com();
                    log_engine_response_move(mv);
                    resp = Some(ComResponse::Move(mv));
                }
                addrs::THINK_END_HUM_WIN => {
                    log_engine_response_hum_win();
                    resp = Some(ComResponse::HumWin);
                }
                addrs::THINK_END_HUM_SUICIDE => {
                    log_engine_response_hum_suicide();
                    resp = Some(ComResponse::HumSuicide);
                }
                addrs::THINK_END_COM_WIN => {
                    let mv = emu::read_move_com();
                    log_engine_response_com_win(mv);
                    resp = Some(ComResponse::ComWin(mv));
                }
                _ => {}
            })?;
        }

        log_think_end();

        let resp = resp.expect("resp should not be None");

        Ok(resp)
    }

    fn run_frame(&mut self, buttons: Buttons) -> anyhow::Result<()> {
        self.run_frame_hooked(buttons, &|_| {})
    }

    fn run_frame_hooked(
        &mut self,
        buttons: Buttons,
        f_hook: &dyn FnMut(u16),
    ) -> anyhow::Result<()> {
        // 閉じるボタンが押されたら強制終了する。
        for ev in self.event_pump.poll_iter() {
            #[allow(clippy::single_match)]
            match ev {
                Event::Quit { .. } => std::process::exit(1),
                _ => {}
            }
        }

        self.tex
            .with_lock(None, |buf, pitch| {
                emu::run_frame_hooked(
                    buttons,
                    |xbuf, _| {
                        for y in 0..240 {
                            for x in 0..256 {
                                let i = pitch * y + 4 * x;
                                let (r, g, b) = emu::nes_color(xbuf[256 * y + x]);
                                buf[i] = 0;
                                buf[i + 1] = b;
                                buf[i + 2] = g;
                                buf[i + 3] = r;
                            }
                        }
                    },
                    f_hook,
                );
            })
            .map_err(|e| anyhow!(e))?;

        self.canvas
            .copy(&self.tex, None, None)
            .map_err(|e| anyhow!(e))?;
        self.canvas.present();

        Ok(())
    }
}

/// 思考ルーチンの応答。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ComResponse {
    Move(Move),
    HumWin,
    HumSuicide,
    ComWin(Move),
}
