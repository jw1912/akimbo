use crate::lsb;

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
        let mut seed: u64 = 180_620_142;
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
