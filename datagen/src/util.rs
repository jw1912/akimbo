use std::{sync::atomic::{AtomicU64, Ordering::Relaxed}, time::Instant};

use akimbo::{consts::Side, position::Position};

use crate::ansi;

pub fn to_fen(pos: &Position, score: i32) -> String {
    const PIECES: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
    let mut fen = String::new();

    for rank in (0..8).rev() {
        let mut clear = 0;

        for file in 0..8 {
            let sq = 8 * rank + file;
            let bit = 1 << sq;
            let pc = pos.get_pc(bit);
            if pc != 0 {
                if clear > 0 {
                    fen.push_str(&format!("{}", clear));
                }
                clear = 0;
                fen.push(PIECES[pc - 2 + 6 * usize::from(pos.side(Side::BLACK) & bit > 0)]);
            } else {
                clear += 1;
            }
        }

        if clear > 0 {
            fen.push_str(&format!("{}", clear));
        }

        if rank > 0 {
            fen.push('/');
        }
    }

    fen.push(' ');
    fen.push(['w', 'b'][pos.stm()]);
    fen.push_str(" - - 0 1 | ");
    fen.push_str(&if pos.stm() > 0 { -score } else { score }.to_string());

    fen
}

pub fn is_terminal(pos: &Position) -> bool {
    let moves = pos.movegen::<true>();
    for &mov in moves.iter() {
        let mut new = *pos;
        if !new.make(mov) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;
    use akimbo::position::Position;

    #[test]
    fn to_fen_test() {
        let pos = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_eq!(
            to_fen(&pos, pos.eval()),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1 6"
        );
    }
}

pub fn update_display(timer: Instant, games: &[AtomicU64], fens: &[AtomicU64]) {
    let elapsed = timer.elapsed().as_secs_f32();
    println!("\x1b[2J\x1b[H");
    println!("+--------+-------------+--------------+--------------+");
    println!("| Thread |    Games    |     Fens     |   Fens/Sec   |");
    println!("+--------+-------------+--------------+--------------+");

    for (i, (num_games, num_fens)) in games.iter().zip(fens.iter()).enumerate() {
        let ng = num_games.load(Relaxed);
        let nf = num_fens.load(Relaxed);
        let fs = nf as f32 / elapsed;
        println!(
            "| {} | {} | {} | {} |",
            ansi!(format!("{i:^6}"), 36),
            ansi!(format!("{ng:^11}"), 36),
            ansi!(format!("{nf:^12}"), 36),
            ansi!(format!("{fs:^12.0}"), 36),
        );
    }

    println!("+--------+-------------+--------------+--------------+");
}

#[macro_export]
macro_rules! ansi {
    ($x:expr, $y:expr) => {
        format!("\x1b[{}m{}\x1b[0m", $y, $x)
    };
    ($x:expr, $y:expr, $esc:expr) => {
        format!("\x1b[{}m{}\x1b[0m{}", $y, $x, $esc)
    };
}
