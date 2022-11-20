use super::{consts::{MATE_THRESHOLD, MAX_PLY}, search::PLY};

/// Hash Table
pub static mut TT: Vec<[HashEntry; 8]> = Vec::new();
/// Number of **buckets** in the transposition table
static mut TT_SIZE: usize = 0;

/// Killer Move Table
pub static mut KT: [[u16; 3]; MAX_PLY as usize] = [[0; 3]; MAX_PLY as usize];

/// The type of bound determined by the hash entry when it was searched.
pub struct Bound;
impl Bound {
    pub const LOWER: u8 = 1;
    pub const UPPER: u8 = 2;
    pub const EXACT: u8 = 3;
}

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    pub key: u16,
    pub best_move: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
}

/// Resizes the hash table to given size **in megabytes**, rounded down to nearest power of 2.
pub fn tt_resize(mut size: usize) {
    unsafe {
    size = 2usize.pow((size as f64).log2().floor() as u32);
    TT_SIZE = size * 1024 * 1024 / std::mem::size_of::<[HashEntry; 8]>();
    TT = vec![Default::default(); TT_SIZE];
    }
}

pub fn tt_clear() {
    unsafe {TT = vec![Default::default(); TT_SIZE]}
}

/// Push a search result to the hash table.
/// #### Replacement Scheme
/// 1. Prioritise replacing entries for the same position (key) that have lower depth.
/// 2. Fill empty entries in bucket.
/// 3. Replace lowest depth entry in bucket.
pub fn tt_push(zobrist: u64, best_move: u16, depth: i8, bound: u8, mut score: i16) {
    unsafe {
    let key: u16 = (zobrist >> 48) as u16;
    let idx: usize = (zobrist as usize) & (TT.len() - 1);
    let bucket: &mut [HashEntry] = &mut TT[idx];
    let mut desired_idx: usize = usize::MAX;
    let mut smallest_depth: i8 = i8::MAX;
    for (entry_idx, &entry) in bucket.iter().enumerate() {
        if (entry.key == key && depth > entry.depth) || entry.depth == 0 {
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
    bucket[desired_idx] = HashEntry {key, best_move, depth, bound, score };
    }
}

pub fn tt_probe(zobrist: u64) -> Option<HashEntry> {
    unsafe{
    let key: u16 = (zobrist >> 48) as u16;
    let idx: usize = (zobrist as usize) & (TT.len() - 1);
    let bucket: &[HashEntry; 8] = &TT[idx];
    for entry in bucket {
        if entry.key == key {
            let mut res: HashEntry = *entry;
            if res.score > MATE_THRESHOLD {
                res.score -= PLY as i16;
            } else if res.score < -MATE_THRESHOLD {
                res.score += PLY as i16;
            }
            return Some(res);
        }
    }}
    None
}

pub fn kt_push(m: u16) {
    unsafe {
    let ply: usize = PLY as usize - 1;
    let new: u16 = if KT[ply].contains(&m) {KT[ply][2]} else {m};
    KT[ply][2] = KT[ply][1];
    KT[ply][1] = KT[ply][0];
    KT[ply][0] = new;
    }
}

pub fn kt_clear() {
    unsafe{KT = [[0; 3]; MAX_PLY as usize]}
}
