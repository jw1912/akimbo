use crate::{consts::Piece, position::Position};

const DI: usize = 12;
const DK: usize = 32;
const DV: usize = 8;

const D1: usize = 16;

static NETWORK: Network = unsafe {
    std::mem::transmute(*include_bytes!("../resources/network-16.bin"))
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Network {
    wq: [[[f32; DK]; DI]; 64],
    wk: [[[f32; DK]; DI]; 64],
    wv: [[[f32; DV]; DI]; 64],
    l1w: [[[f32; D1]; DV]; 64],
    l1b: [f32; D1],
    l2w: [f32; D1],
    l2b: f32,
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

    let mut hl = [[0.0; DV]; 32];

    for i in 0..num_pieces {
        let mut temps = [0.0; 32];
        let mut max = 0f32;

        for j in 0..num_pieces {
            let query = &NETWORK.wq[squares[i]][pieces[i]];
            let key = &NETWORK.wk[squares[j]][pieces[j]];

            for k in 0..DK {
                temps[j] += query[k] * key[k]
            }

            max = max.max(temps[j]);
        }

        let mut total = (64 - num_pieces) as f32 * (-max).exp();

        for t in temps.iter_mut().take(num_pieces) {
            *t = (*t - max).exp();
            total += *t;
        }

        for j in 0..num_pieces {
            temps[j] /= total;

            let value = &NETWORK.wv[squares[j]][pieces[j]];
            let weight = temps[j];
    
            for (k, &val) in value.iter().enumerate() {
                hl[i][k] += weight * val;
            }
        }
    }

    let mut l1 = NETWORK.l1b;

    for i in 0..num_pieces {
        let weights = &NETWORK.l1w[squares[i]];

        for j in 0..DV {
            hl[i][j] = hl[i][j].max(0.0);
        }

        for (j, w) in weights.iter().enumerate() {
            for k in 0..D1 {
                l1[k] += hl[i][j].max(0.0) * w[k];
            }
        }
    }

    let mut l2 = NETWORK.l2b;

    for (&w, &n) in NETWORK.l2w.iter().zip(l1.iter()) {
        l2 += w * n.max(0.0);
    }

    (l2 * 400.0) as i32
}
