use criterion::{black_box, criterion_group, criterion_main, Criterion};

use naitou_clone::*;

criterion_group!(benches, bench);
criterion_main!(benches);

pub fn bench(c: &mut Criterion) {
    bbs::init();

    c.bench_function("naitou_perft", |b| b.iter(|| naitou_perft(black_box(2))));
}

fn naitou_perft(depth: u32) -> u64 {
    let (mut engine, _) = Engine::new(Handicap::HumSenteSikenbisha);

    naitou_perft_rec(&mut engine, depth)
}

fn naitou_perft_rec(engine: &mut Engine, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let pos = engine.position();

    let mvs = if pos.is_checked(HUM) {
        generate_evasions(pos)
    } else {
        generate_moves(pos)
    };

    let mut res = 0;

    for mv in mvs {
        if let Ok(resp) = engine.do_step(mv) {
            match &resp {
                EngineResponse::Move(_) => {
                    res += naitou_perft_rec(engine, depth - 1);
                }
                _ => {}
            }
            engine.undo_step(&resp);
        }
    }

    res
}
