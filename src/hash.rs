use super::consts::*;

pub static mut TT: Vec<HashBucket> = Vec::new();

pub struct Bound;
impl Bound {
    pub const INVALID: u8 = 0;
    pub const LOWER: u8 = 1;
    pub const UPPER: u8 = 2;
    pub const EXACT: u8 = 3;
}

#[derive(Clone, Copy, Default)]
#[repr(align(64))]
pub struct HashBucket(pub [u64; 8]);

const BUCKET_SIZE: usize = std::mem::size_of::<HashBucket>();
#[derive(Default)]
pub struct HashResult {
    pub key: u16,
    pub best_move: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
}

pub fn tt_resize(size: usize) {
    unsafe {TT = vec![Default::default(); size / BUCKET_SIZE] }
}

pub fn tt_clear() {
    unsafe {
        for bucket in TT.iter_mut() {
            *bucket = Default::default();
        }
    }
}

fn tt_load(data: u64) -> HashResult {
    HashResult {
        key: data as u16,
        best_move: (data >> 16) as u16,
        score: (data >> 32) as i16,
        depth: (data >> 48) as i8,
        bound: ((data >> 56) & 3) as u8,
    }
}

fn tt_encode(key: u16, best_move: u16, depth: i8, bound: u8, score: i16) -> u64 {
    (key as u64)
    | ((best_move as u64) << 16)
    | (((score as u16) as u64) << 32)
    | ((depth as u64) << 48)
    | ((bound as u64) << 56)
}

pub fn tt_push(zobrist: u64, best_move: u16, depth: i8, bound: u8, mut score: i16, ply: i8) {
    unsafe {
    let key = (zobrist >> 48) as u16;
    let idx = (zobrist as usize) % TT.len();
    let bucket = &mut TT[idx];
    let mut desired_idx = usize::MAX;
    let mut smallest_depth = i8::MAX;
    for (entry_idx, &entry) in bucket.0.iter().enumerate() {
        let entry_data = tt_load(entry);
        if (entry_data.key == key && depth > entry_data.depth) || entry_data.depth == 0 {
            desired_idx = entry_idx;
            break;
        }
        if entry_data.depth < smallest_depth {
            smallest_depth = entry_data.depth;
            desired_idx = entry_idx;
            continue;
        }
    }
    if score > MATE_THRESHOLD {
        score += ply as i16;
    } else if score < -MATE_THRESHOLD {
        score -= ply as i16;
    }
    bucket.0[desired_idx] = tt_encode(key, best_move, depth, bound, score);
    }
}

pub fn tt_probe(zobrist: u64, ply: i8) -> Option<HashResult> {
    let key = (zobrist >> 48) as u16;
    let idx = (zobrist as usize) % unsafe{TT.len()};
    let bucket = unsafe{&TT[idx]};
    for &data in &bucket.0 {
        if data as u16 == key {
            let mut entry_data = tt_load(data);
            if entry_data.score > MATE_THRESHOLD {
                entry_data.score -= ply as i16;
            } else if entry_data.score < -MATE_THRESHOLD {
                entry_data.score += ply as i16;
            }
            return Some(entry_data);
        } 
    }
    None
}

pub mod zobrist {
    use lazy_static::lazy_static;
    use fastrand;
    use crate::{lsb, pop, position::POS};

    lazy_static!(pub static ref ZVALS: ZobristVals = ZobristVals::init(););

    pub struct ZobristVals {
        pub pieces: [[[u64; 64]; 6]; 2],
        pub castle: [u64; 4],
        pub en_passant: [u64; 8],
        pub side: u64,
    }

    impl ZobristVals {
        #[inline(always)]
        pub fn piece_hash(&self, idx: usize, side: usize, piece: usize) -> u64 {
            self.pieces[side][piece][idx]
        }
        #[inline(always)]
        pub fn castle_hash(&self, current: u8, update: u8) -> u64 {
            if current & update == 0 { return 0 }
            self.castle[lsb!(update as u64) as usize]
        }

        fn init() -> Self {
            fastrand::seed(353012);
            let mut vals = Self {
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

    pub fn calc() -> u64 {
        unsafe {
        let mut zobrist = 0;
        for (i, side) in POS.sides.iter().enumerate() {
            for (j, &pc) in POS.pieces.iter().enumerate() {
                let mut piece = pc & side;
                while piece > 0 {
                    let idx = lsb!(piece) as usize;
                    zobrist ^= ZVALS.pieces[i][j][idx];
                    pop!(piece)
                }
            }
        }
        let mut castle_rights = POS.state.castle_rights;
        while castle_rights > 0 {
            let ls1b = castle_rights & castle_rights.wrapping_neg();
            zobrist ^= ZVALS.castle_hash(0b1111, ls1b);
            pop!(castle_rights)
        }
        if POS.state.en_passant_sq > 0 {zobrist ^= ZVALS.en_passant[(POS.state.en_passant_sq & 7) as usize]}
        if POS.side_to_move == 0 {zobrist ^= ZVALS.side;}
        zobrist
        }
    }
}