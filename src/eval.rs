use crate::{consts::Piece, position::Position};

const EMBED: usize = 32;
const TYPES: usize = 12;

static NETWORK: Network = unsafe {
    std::mem::transmute(*include_bytes!("../resources/network-5.bin"))
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Network {
    wq: [[[f32; EMBED]; TYPES]; 64],
    wv: [[[f32; EMBED]; TYPES]; 64],
    wk: [[[f32; EMBED]; TYPES]; 64],
    outw: [f32; EMBED],
    outb: f32,
}

pub fn eval(pos: &Position) -> i32 {
    let mut num_pieces = 0;

    let mut squares = [0; 32];
    let mut pieces = [0; 32];

    let flip = if pos.stm() > 0 { 56 } else { 0 };

    for (stm, &side) in [pos.stm(), 1 - pos.stm()].iter().enumerate() {
        for piece in Piece::PAWN..=Piece::KING {
            let mut bb = pos.side(side) & pos.piece(piece);

            while bb > 0 {
                let sq = bb.trailing_zeros() as usize;

                squares[num_pieces] = sq ^ flip;
                pieces[num_pieces] = 6 * stm + piece - 2;
                num_pieces += 1;

                bb &= bb - 1;
            }
        }
    }

    let mut logit_sums = [0.0; 32];

    for i in 0..num_pieces {
        let mut temps = [0.0f32; 32];
        let mut total = 0.0;

        for j in 0..num_pieces {
            let query = &NETWORK.wq[squares[i]][pieces[i]];
            let key = &NETWORK.wk[squares[j]][pieces[j]];

            for k in 0..EMBED {
                temps[j] += query[k] * key[k]
            }

            temps[j] = temps[j].exp();
            total += temps[j];
        }

        for j in 0..num_pieces {
            logit_sums[j] += temps[j] / total;
        }
    }

    let mut hl = [0.0; EMBED];

    for i in 0..num_pieces {
        let value = &NETWORK.wv[squares[i]][pieces[i]];
        let weight = logit_sums[i];

        for j in 0..EMBED {
            hl[j] += weight * value[j];
        }
    }

    let mut out = NETWORK.outb;

    for (&n, &w) in hl.iter().zip(NETWORK.outw.iter()) {
        out += w * n.max(0.0);
    }

    (out * 400.0) as i32
}
