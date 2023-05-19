use crate::{util::{HISTORY_MAX, KILLERS, MAX_PLY, Score}, position::Move};

#[derive(Clone, Copy, Default)]
pub struct HashEntry {
    pub key: u16,
    pub best_move: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
}

#[derive(Default)]
pub struct HashTable(Vec<HashEntry>, u8);

impl HashTable {
    pub fn resize(&mut self, size: usize) {
        self.0 = vec![Default::default(); 1 << (80 - (size as u64).leading_zeros())];
        self.1 = 0;
    }

    pub fn clear(&mut self) {
        self.0.iter_mut().for_each(|bucket| *bucket = Default::default());
        self.1 = 0;
    }

    pub fn age(&mut self) {
        self.1 = 63.min(self.1 + 1);
    }

    pub fn push(&mut self, hash: u64, m: Move, depth: i8, bound: u8, mut score: i16, ply: i16) {
        let (key, idx) = ((hash >> 48) as u16, (hash as usize) & (self.0.len() - 1));
        let entry = &mut self.0[idx];
        let diff = self.1 - (entry.bound >> 2);
        if ply > 0 && key == entry.key && depth as u8 + 2 * diff < entry.depth as u8  { return }
        score += if score.abs() > Score::MATE {score.signum() * ply} else {0};
        let best_move = (m.from as u16) << 6 | m.to as u16 | (m.flag as u16) << 12;
        *entry = HashEntry { key, best_move, score, depth, bound: (self.1 << 2) | bound };
    }

    pub fn probe(&self, hash: u64, ply: i16) -> Option<HashEntry> {
        let mut entry = self.0[(hash as usize) & (self.0.len() - 1)];
        if entry.key != (hash >> 48) as u16 { return None }
        entry.score -= if entry.score.abs() > Score::MATE {entry.score.signum() * ply} else {0};
        Some(entry)
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
        self.0[ply][1] = self.0[ply][0];
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
        self.1 = 1.max(self.1 / 64);
        for side in self.0.iter_mut() {
            for pc in side.iter_mut() {
                pc.iter_mut().for_each(|sq| *sq /= 64);
            }
        }
    }

    pub fn push(&mut self, mov: Move, side: bool, depth: i8) {
        let entry = &mut self.0[usize::from(side)][usize::from(mov.pc - 2)][usize::from(mov.to)];
        *entry += (depth as i64).pow(2);
        self.1 = self.1.max(*entry);
    }

    pub fn score(&self, mov: Move, side: bool) -> i16 {
        let entry = self.0[usize::from(side)][usize::from(mov.pc - 2)][usize::from(mov.to)];
        ((HISTORY_MAX * entry + self.1 - 1) / self.1) as i16
    }
}