use crate::{consts::*, position::Move};

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    pub key: u16,
    pub best_move: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
}

#[derive(Default)]
pub struct HashTable {
    table: Vec<[HashEntry; 8]>,
    num_buckets: usize,
}

impl HashTable {
    pub fn resize(&mut self, mut size: usize) {
        size = 2usize.pow((size as f64).log2().floor() as u32);
        self.num_buckets = size * 1024 * 1024 / std::mem::size_of::<[HashEntry; 8]>();
        self.table = vec![Default::default(); self.num_buckets];
    }

    pub fn clear(&mut self) {
        self.table.iter_mut().for_each(|bucket: &mut [HashEntry; 8]| *bucket = [HashEntry::default(); 8]);
    }

    pub fn push(&mut self, zobrist: u64, m: Move, depth: i8, bound: u8, mut score: i16, ply: i16) {
        let key = (zobrist >> 48) as u16;
        let idx = (zobrist as usize) & (self.num_buckets- 1);
        let mut desired_idx = usize::MAX;
        let mut smallest_depth = i8::MAX;
        for (entry_idx, entry) in self.table[idx].iter().enumerate() {
            if (entry.key == key && depth > entry.depth) || entry.depth == 0 {
                desired_idx = entry_idx;
                break;
            }
            if entry.depth < smallest_depth {
                smallest_depth = entry.depth;
                desired_idx = entry_idx;
            }
        }
        score += if score > MATE {ply} else if score < -MATE {-ply} else {0};
        let best_move = (m.from as u16) << 6 | m.to as u16 | (m.flag as u16) << 12;
        self.table[idx][desired_idx] = HashEntry { key, best_move, score, depth, bound };
    }

    pub fn probe(&self, zobrist: u64, ply: i16) -> Option<HashEntry> {
        let key = (zobrist >> 48) as u16;
        let idx = (zobrist as usize) & (self.num_buckets - 1);
        for entry in &self.table[idx] {
            if entry.key == key {
                let mut res = *entry;
                res.score += if res.score > MATE {-ply} else if res.score < -MATE {ply} else {0};
                return Some(res)
            }
        }
        None
    }
}

pub struct KillerTable(pub [[Move; KILLERS]; MAX_PLY as usize + 1]);

impl Default for KillerTable {
    fn default() -> Self {
        Self([Default::default(); MAX_PLY as usize + 1])
    }
}

impl KillerTable {
    pub fn push(&mut self, m: Move, p: i16) {
        let ply = p as usize - 1;
        (0..{KILLERS - 1}).rev().for_each(|i: usize| self.0[ply][i + 1] = self.0[ply][i]);
        self.0[ply][0] = m;
    }
}

pub struct HistoryTable([[[i64; 64]; 6]; 2], i64);

impl Default for HistoryTable {
    fn default() -> Self {
        Self([[[0; 64]; 6]; 2], 1)
    }
}

impl HistoryTable {
    pub fn age(&mut self) {
        self.0.iter_mut().for_each(|side|
            side.iter_mut().for_each(|pc|
                pc.iter_mut().for_each(|sq| *sq /= 64)))
    }

    pub fn change(&mut self, side: bool, m: Move, amount: i64) {
        let entry = &mut self.0[usize::from(side)][usize::from(m.mpc - 2)][usize::from(m.to)];
        *entry += amount;
        self.1 = self.1.max(*entry);
    }

    pub fn score(&self, side: bool, m: Move) -> i16 {
        let entry = self.0[usize::from(side)][usize::from(m.mpc - 2)][usize::from(m.to)];
        ((HISTORY_MAX * entry + self.1 - 1) / self.1) as i16
    }
}