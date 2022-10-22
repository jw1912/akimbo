pub mod consts;
pub mod position;
pub mod movegen;
pub mod hash;
pub mod eval;
pub mod search;

use position::*;
use movegen::*;
use std::time::Instant;

const _POSITIONS: [&str; 2] = [
    // Start Position
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 
    // Kiwipete Position
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
];
fn main() {
    for (i, fen) in _POSITIONS.iter().enumerate() {
        parse_fen(fen);
        let mut t = 0;
        let time = Instant::now();
        for d in 1..(7 - i as u8) {
            let p = perft(d);
            t += p;
            println!("info depth {} nodes {} time {}", d, p, time.elapsed().as_millis());
        }
        let f = time.elapsed().as_millis();
        println!("info time {} nps {}", f, t * 1000 / f as u64)
    }
}

pub fn perft(depth_left: u8) -> u64 {
    if depth_left == 0 { return 1 }
    let mut moves = MoveList::default();
    gen_moves::<All>(&mut moves);
    let mut positions: u64 = 0;
    for m_idx in 0..moves.len {
        let m = moves.list[m_idx];
        let ctx = do_move(m);
        if ctx.invalid { continue }
        let score = perft(depth_left - 1);
        positions += score;
        undo_move(ctx);
    }
    positions
}