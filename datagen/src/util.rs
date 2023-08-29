use akimbo::position::{Move, Position};

pub fn is_capture(mov: Move) -> bool {
    mov.flag & 4 > 0
}

pub fn to_fen(pos: &Position, score: i32) -> String {
    const PIECES: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
    let mut fen = String::new();

    for rank in (0..8).rev() {
        let mut clear = 0;

        for file in 0..8{
            let sq = 8 * rank + file;
            let bit = 1 << sq;
            let pc = pos.get_pc(bit);
            if pc != 0 {
                if clear > 0 {
                    fen.push_str(&format!("{}", clear));
                }
                clear = 0;
                fen.push(PIECES[pc - 2 + 6 * usize::from(pos.bb[1] & bit > 0)]);
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
    fen.push(['w', 'b'][usize::from(pos.c)]);
    fen.push_str(" - - 0 1");
    fen.push_str(&format!(" {}", if pos.c {-score} else {score}));

    fen
}

pub fn is_terminal(pos: &Position) -> bool {
    let moves = pos.movegen::<true>();
    for &mov in &moves.list[..moves.len] {
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
        assert_eq!(to_fen(&pos, pos.eval()), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1 6");
    }
}