use super::{consts::{MATE_THRESHOLD, MAX_PLY}, search::PLY};

/// HASH TABLE
pub static mut TT: Vec<HashBucket> = Vec::new();
/// Number of **buckets** in the transposition table
static mut TT_SIZE: usize = 0;
/// Number of **entries** filled
static mut FILLED: u64 = 0;

/// Killer Move Table
pub static mut KT: [[u16; 3]; MAX_PLY as usize] = [[0; 3]; MAX_PLY as usize];

/// The type of bound determined by the hash entry when it was searched.
pub struct Bound;
impl Bound {
    /// Best score >= beta.
    pub const LOWER: u8 = 1;
    /// Best score < alpha.
    pub const UPPER: u8 = 2;
    /// Best score between alpha and beta.
    pub const EXACT: u8 = 3;
}

/// A 64 byte-aligned bucket that can hold up to 8 entries
/// with the same hash key modulo the number of buckets in
/// the hash table.
#[derive(Clone, Copy, Default)]
#[repr(align(64))]
pub struct HashBucket(pub [HashEntry; 8]);

/// Split of the encoded entries into their constituent parts.
#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    /// Last 16 bits of the zobrist hash for the position.
    pub key: u16,
    /// Hash move.
    pub best_move: u16,
    /// Hash score.
    pub score: i16,
    /// Depth of search that determined this entry.
    pub depth: i8,
    /// Bound type.
    pub bound: u8,
}

/// The proportion of the hash table that is filled,
/// measured in permill.
pub fn hashfull() -> u64 {
    unsafe {FILLED * 1000 / (8 * TT_SIZE) as u64}
}

/// Resizes the hash table to given size **in megabytes**, rounded down to nearest power of 2.
pub fn tt_resize(mut size: usize) {
    unsafe {
        size = 2usize.pow((size as f64).log2().floor() as u32);
        TT_SIZE = size * 1024 * 1024 / std::mem::size_of::<HashBucket>();
        TT = vec![Default::default(); TT_SIZE];
        FILLED = 0;
    }
}

/// Clears the hash table.
pub fn tt_clear() {
    unsafe {
        TT = vec![Default::default(); TT_SIZE];
        FILLED = 0;
    }
}

/// Push a search result to the hash table.
/// #### Replacement Scheme
/// 1. Prioritise replacing entries for the same position (key) that have lower depth.
/// 2. Fill empty entries in bucket.
/// 3. Replace lowest depth entry in bucket.
pub fn tt_push(zobrist: u64, best_move: u16, depth: i8, bound: u8, mut score: i16) {
    unsafe {
    let key: u16 = (zobrist >> 48) as u16;
    let idx: usize = (zobrist as usize) % TT.len();
    let bucket: &mut HashBucket = &mut TT[idx];
    let mut desired_idx: usize = usize::MAX;
    let mut smallest_depth: i8 = i8::MAX;
    for (entry_idx, &entry) in bucket.0.iter().enumerate() {
        if entry.key == key && depth > entry.depth {
            desired_idx = entry_idx;
            break;
        }
        if entry.depth == 0 {
            FILLED += 1;
            desired_idx = entry_idx;
            break;
        }
        if entry.depth < smallest_depth {
            smallest_depth = entry.depth;
            desired_idx = entry_idx;
            continue;
        }
    }
    if score > MATE_THRESHOLD {
        score += PLY as i16;
    } else if score < -MATE_THRESHOLD {
        score -= PLY as i16;
    }
    bucket.0[desired_idx] = HashEntry {key, best_move, depth, bound, score };
    }
}

/// Probe the hash table to find an entry with given zobrist key.
pub fn tt_probe(zobrist: u64) -> Option<HashEntry> {
    let key: u16 = (zobrist >> 48) as u16;
    let idx: usize = (zobrist as usize) & (unsafe{TT.len()} - 1);
    let bucket: &HashBucket = unsafe{&TT[idx]};
    for entry in &bucket.0 {
        if entry.key == key {
            let mut res = *entry;
            if res.score > MATE_THRESHOLD {
                res.score -= unsafe{PLY} as i16;
            } else if res.score < -MATE_THRESHOLD {
                res.score += unsafe{PLY} as i16;
            }
            return Some(res);
        }
    }
    None
}

/// Methods and initialisation of zobrist hashing values.
pub mod zobrist {
    use crate::{lsb, pop, position::POS};
    use lazy_static::lazy_static;
    use fastrand;

    lazy_static!(
        /// Zobrist hashing values, initialised on first call.
        pub static ref ZVALS: ZobristVals = ZobristVals::init();
    );

    /// Container for zobrist hashing values.
    pub struct ZobristVals {
        /// Hash value for each piece on each square.
        pub pieces: [[[u64; 64]; 6]; 2],
        /// Castle hash values
        pub castle: [u64; 4],
        /// En passant hash value based on file.
        pub en_passant: [u64; 8],
        /// Side to move hash value
        pub side: u64,
    }

    impl ZobristVals {
        /// Calculates mask for updating castle hash.
        #[inline(always)]
        pub fn castle_hash(&self, current: u8, update: u8) -> u64 {
            if current & update == 0 { return 0 }
            self.castle[lsb!(update as u64) as usize]
        }

        /// Initialises ZVALS.
        fn init() -> Self {
            fastrand::seed(353012);
            let mut vals: ZobristVals = Self {
                pieces: [[[0; 64]; 6]; 2],
                castle: [0; 4],
                en_passant: [0; 8],
                side: fastrand::u64(1..u64::MAX),
            };
            for color in 0..2 {
                for piece in 0..6 {
                    for sq_idx in 0..64 {
                        vals.pieces[color][piece][sq_idx] = fastrand::u64(1..u64::MAX);
                    }
                }
            }
            for idx in 0..4 {vals.castle[idx] = fastrand::u64(1..u64::MAX);}
            for idx in 0..8 {vals.en_passant[idx] = fastrand::u64(1..u64::MAX);}
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
}

/// Push a move to the killer moves table.
pub fn kt_push(m: u16) {
    unsafe {
    let ply: usize = PLY as usize - 1;
    if KT[ply].contains(&m) { return }
    KT[ply][2] = KT[ply][1];
    KT[ply][1] = KT[ply][0];
    KT[ply][0] = m;
    }
}

/// Clear the killer moves table.
pub fn kt_clear() {
    unsafe{KT = [[0; 3]; MAX_PLY as usize]}
}
