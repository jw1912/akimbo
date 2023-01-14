use super::consts::{KILLERS_PER_PLY, MATE, MAX_PLY};

/// The type of bound determined by the hash entry when it was searched.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Bound {#[default] Lower, Upper, Exact}

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    pub key: u16,
    pub best_move: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: Bound,
}

pub struct HashTable {
    table: Vec<[HashEntry; 8]>,
    num_buckets: usize,
}

impl HashTable {
    /// Instantiates a new hash table with size 1mb.
    pub fn new() -> Self {
        let mut ret: Self = Self { table: Vec::new(), num_buckets: 0 };
        ret.resize(1);
        ret
    }

    /// Resizes the hash table to given size **in megabytes**, rounded down to nearest power of 2.
    pub fn resize(&mut self, mut size: usize) {
        size = 2usize.pow((size as f64).log2().floor() as u32);
        self.num_buckets = size * 1024 * 1024 / std::mem::size_of::<[HashEntry; 8]>();
        self.table = vec![Default::default(); self.num_buckets];
    }

    pub fn clear(&mut self) {
        self.table.iter_mut().for_each(|bucket: &mut [HashEntry; 8]| *bucket = [HashEntry::default(); 8]);
    }

    /// Push a search result to the hash table.
    /// #### Replacement Scheme
    /// 1. Prioritise replacing entries for the same position (key) that have lower depth.
    /// 2. Fill empty entries in bucket.
    /// 3. Replace lowest depth entry in bucket.
    pub fn push(&mut self, zobrist: u64, best_move: u16, depth: i8, bound: Bound, mut score: i16, ply: i16) {
        let key: u16 = (zobrist >> 48) as u16;
        let idx: usize = (zobrist as usize) & (self.num_buckets- 1);
        let bucket: &mut [HashEntry] = &mut self.table[idx];
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
        score += if score > MATE {ply} else if score < -MATE {-ply} else {0};
        bucket[desired_idx] = HashEntry { key, best_move, score, depth, bound };
    }

    /// Probes the hash table for an entry matching the provided hash value, returning first match.
    pub fn probe(&self, zobrist: u64, ply: i16) -> Option<HashEntry> {
        let key: u16 = (zobrist >> 48) as u16;
        let idx: usize = (zobrist as usize) & (self.num_buckets - 1);
        let bucket: &[HashEntry; 8] = &self.table[idx];
        for entry in bucket {
            if entry.key == key {
                let mut res: HashEntry = *entry;
                res.score += if res.score > MATE {-ply} else if res.score < -MATE {ply} else {0};
                return Some(res);
            }
        }
        None
    }
}

pub struct KillerTable(pub [[u16; KILLERS_PER_PLY]; MAX_PLY as usize + 1]);
impl KillerTable {
    pub fn push(&mut self, m: u16, p: i16) {
        let ply: usize = p as usize - 1;
        let new: u16 = if self.0[ply].contains(&m) {self.0[ply][KILLERS_PER_PLY - 1]} else {m};
        (0..{KILLERS_PER_PLY - 1}).rev().for_each(|i: usize| self.0[ply][i + 1] = self.0[ply][i]);
        self.0[ply][0] = new;
    }

    pub fn clear(&mut self) {
        self.0.iter_mut().for_each(|bucket: &mut [u16; 3]| *bucket = [0; KILLERS_PER_PLY]);
    }
}

#[derive(Clone, Copy, Default)]
pub struct HistoryScore(pub u64, pub u64);
