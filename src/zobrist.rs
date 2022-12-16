use crate::{lsb, pop, position::POS};

pub static mut ZVALS: ZobristVals = ZobristVals {pieces: [[[0; 64]; 6]; 2], castle: [0; 4], en_passant: [0; 8], side: 0};

pub struct ZobristVals {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub castle: [u64; 4],
    pub en_passant: [u64; 8],
    pub side: u64,
}

fn xor_shift(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

impl ZobristVals {
    /// Calculates mask for updating castle hash.
    #[inline(always)]
    pub fn castle_hash(&self, current: u8, update: u8) -> u64 {
        if current & update == 0 { return 0 }
        self.castle[lsb!(update as u64) as usize]
    }
    /// Initialises ZVALS.
    pub fn init() -> Self {
        let mut seed: u64 = 180620142;
        let mut vals: ZobristVals = Self {
            pieces: [[[0; 64]; 6]; 2],
            castle: [0; 4],
            en_passant: [0; 8],
            side: xor_shift(&mut seed),
        };
        for color in 0..2 {
            for piece in 0..6 {
                for sq_idx in 0..64 {
                    vals.pieces[color][piece][sq_idx] = xor_shift(&mut seed);
                }
            }
        }
        for idx in 0..4 {vals.castle[idx] = xor_shift(&mut seed);}
        for idx in 0..8 {vals.en_passant[idx] = xor_shift(&mut seed);}
        vals
    }
}

/// Calculate the zobrist hash value for the current position, from scratch.
pub fn calc() -> u64 {
    unsafe {
    let mut zobrist: u64 = 0;
    for (i, side) in POS.sides.iter().enumerate() {
        for (j, &pc) in POS.pieces.iter().enumerate() {
            let mut piece: u64 = pc & side;
            while piece > 0 {
                let idx: usize = lsb!(piece) as usize;
                zobrist ^= ZVALS.pieces[i][j][idx];
                pop!(piece)
            }
        }
    }
    let mut castle_rights: u8 = POS.state.castle_rights;
    while castle_rights > 0 {
        let ls1b: u8 = castle_rights & castle_rights.wrapping_neg();
        zobrist ^= ZVALS.castle_hash(0b1111, ls1b);
        pop!(castle_rights)
    }
    if POS.state.en_passant_sq > 0 {zobrist ^= ZVALS.en_passant[(POS.state.en_passant_sq & 7) as usize]}
    if POS.side_to_move == 0 {zobrist ^= ZVALS.side;}
    zobrist
    }
}