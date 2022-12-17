use crate::{lsb, pop, position::Position};

pub static ZVALS: ZobristVals = ZobristVals::init();

/// Holds random hash values for each aspect of the board position
pub struct ZobristVals {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub castle: [u64; 4],
    pub en_passant: [u64; 8],
    pub side: u64,
}

/// Simple pseudo-random number generator
const fn xor_shift(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

impl ZobristVals {
    /// Calculates mask for updating castle hash.
    #[inline(always)]
    pub fn castle_hash(&self, current: u8, update: u8) -> u64 {
        if current & update == 0 { return 0 }
        self.castle[lsb!(update) as usize]
    }

    /// Initialises ZVALS.
    pub const fn init() -> Self {
        let mut seed: u64 = 180620142;
        seed = xor_shift(seed);
        let mut vals: ZobristVals = Self {
            pieces: [[[0; 64]; 6]; 2],
            castle: [0; 4],
            en_passant: [0; 8],
            side: seed,
        };
        let mut idx: usize = 0;
        while idx < 2 {
            let mut piece: usize = 0;
            while piece < 6 {
                let mut square: usize = 0;
                while square < 64 {
                    seed = xor_shift(seed);
                    vals.pieces[idx][piece][square] = seed;
                    square += 1;
                }
                piece += 1;
            }
            idx += 1;
        }
        while idx < 6 {seed = xor_shift(seed); vals.castle[idx - 2] = seed; idx += 1;}
        while idx < 14 {seed = xor_shift(seed); vals.en_passant[idx - 6] = seed; idx += 1;}
        vals
    }
}

impl Position {
    /// Calculate the zobrist hash value for the current position, from scratch.
    pub fn hash(&self) -> u64 {
        let mut zobrist: u64 = 0;
        for (i, side) in self.sides.iter().enumerate() {
            for (j, &pc) in self.pieces.iter().enumerate() {
                let mut piece: u64 = pc & side;
                while piece > 0 {
                    let idx: usize = lsb!(piece) as usize;
                    zobrist ^= ZVALS.pieces[i][j][idx];
                    pop!(piece)
                }
            }
        }
        let mut castle_rights: u8 = self.state.castle_rights;
        while castle_rights > 0 {
            let ls1b: u8 = castle_rights & castle_rights.wrapping_neg();
            zobrist ^= ZVALS.castle_hash(0b1111, ls1b);
            pop!(castle_rights)
        }
        if self.state.en_passant_sq > 0 {zobrist ^= ZVALS.en_passant[(self.state.en_passant_sq & 7) as usize]}
        if self.side_to_move == 0 {zobrist ^= ZVALS.side;}
        zobrist
    }
}