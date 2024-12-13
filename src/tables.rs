use std::sync::atomic::{AtomicU64, AtomicU8, Ordering::Relaxed};

use crate::{
    consts::{CorrectionHistory, MoveScore, Score},
    moves::{Move, MoveList},
    position::Position,
    util::boxed_and_zeroed,
};

pub struct HashView<'a> {
    table: &'a HashTable,
}

impl std::ops::Deref for HashView<'_> {
    type Target = HashTable;
    fn deref(&self) -> &Self::Target {
        self.table
    }
}

impl<'a> HashView<'a> {
    pub fn new(tt: &'a HashTable) -> Self {
        Self { table: tt }
    }

    pub fn prefetch(&self, hash: u64) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

            let idx = (hash as usize) & (self.table.table.len() - 1);
            let entry = &self.table.table[idx];
            let ptr = entry as *const HashEntryInternal;

            _mm_prefetch::<_MM_HINT_T0>(ptr.cast());
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct HashEntry {
    key: u16,
    best_move: u16,
    score: i16,
    depth: i8,
    bound: u8,
}

impl HashEntry {
    pub fn depth(&self) -> i32 {
        i32::from(self.depth)
    }

    pub fn bound(&self) -> u8 {
        self.bound & 3
    }

    pub fn score(&self) -> i32 {
        i32::from(self.score)
    }

    pub fn best_move(&self, pos: &Position) -> Move {
        Move::from_short(self.best_move, pos)
    }

    pub fn from_internal(atom: HashEntryInternal) -> Self {
        unsafe { std::mem::transmute(atom.data.load(Relaxed)) }
    }

    pub fn to_u64(self) -> u64 {
        unsafe { std::mem::transmute(self) }
    }
}

#[derive(Default)]
pub struct HashEntryInternal {
    data: AtomicU64,
}

impl Clone for HashEntryInternal {
    fn clone(&self) -> Self {
        Self {
            data: AtomicU64::new(self.data.load(Relaxed)),
        }
    }
}

#[derive(Default)]
pub struct HashTable {
    table: Vec<HashEntryInternal>,
    age: AtomicU8,
}

impl HashTable {
    pub fn resize(&mut self, size: usize, threads: usize) {
        self.age.store(0, Relaxed);

        let num_entries = 1 << (80 - (size as u64).leading_zeros());
        let chunk_size = num_entries / threads + 1;

        self.table = Vec::new();
        self.table.reserve_exact(num_entries);

        unsafe {
            use std::mem::{size_of, MaybeUninit};
            let ptr = self.table.as_mut_ptr().cast();
            let uninit: &mut [MaybeUninit<u8>] =
                std::slice::from_raw_parts_mut(ptr, num_entries * size_of::<HashEntryInternal>());

            std::thread::scope(|s| {
                for chunk in uninit.chunks_mut(chunk_size) {
                    s.spawn(|| {
                        chunk.as_mut_ptr().write_bytes(0, chunk.len());
                    });
                }
            });

            self.table.set_len(num_entries);
        }
    }

    pub fn clear(&mut self, threads: usize) {
        self.age.store(0, Relaxed);

        let chunk_size = self.table.len() / threads + 1;

        std::thread::scope(|s| {
            for chunk in self.table.chunks_mut(chunk_size) {
                s.spawn(|| {
                    for entry in chunk.iter_mut() {
                        *entry = HashEntryInternal::default();
                    }
                });
            }
        });
    }

    pub fn age_up(&self) {
        self.age.store(63.min(self.get_age() + 1), Relaxed);
    }

    pub fn get_age(&self) -> u8 {
        self.age.load(Relaxed)
    }

    pub fn push(&self, hash: u64, mov: Move, depth: i8, bound: u8, mut score: i32, ply: i32) {
        let key = (hash >> 48) as u16;
        let idx = (hash as usize) & (self.table.len() - 1);
        let entry = HashEntry::from_internal(self.table[idx].clone());

        // replacement scheme
        let diff = self.get_age() - (entry.bound >> 2);
        if ply > 0 && key == entry.key && depth as u8 + 2 * diff < entry.depth as u8 {
            return;
        }

        // replace entry
        score += if score.abs() > Score::MATE {
            score.signum() * ply
        } else {
            0
        };
        let best_move = mov.to_short();
        let new_entry = HashEntry {
            key,
            best_move,
            score: score as i16,
            depth,
            bound: (self.get_age() << 2) | bound,
        }
        .to_u64();

        self.table[idx].data.store(new_entry, Relaxed);
    }

    pub fn probe(&self, hash: u64, ply: i32) -> Option<HashEntry> {
        let idx = (hash as usize) & (self.table.len() - 1);
        let mut entry = HashEntry::from_internal(self.table[idx].clone());

        if entry.key != (hash >> 48) as u16 {
            return None;
        }

        entry.score -= if entry.score.abs() > Score::MATE as i16 {
            entry.score.signum() * ply as i16
        } else {
            0
        };

        Some(entry)
    }
}

#[derive(Copy, Clone)]
pub struct HistoryEntry {
    score: [[i32; 2]; 2],
    continuation: [[i32; 64]; 6],
}

impl Default for HistoryEntry {
    fn default() -> Self {
        Self {
            score: [[0; 2]; 2],
            continuation: [[0; 64]; 6],
        }
    }
}

#[derive(Clone)]
pub struct HistoryTable {
    table: Box<[[[HistoryEntry; 64]; 8]; 2]>,
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self {
            table: boxed_and_zeroed(),
        }
    }
}

fn threatened(sq: usize, threats: u64) -> usize {
    usize::from(threats & (1 << sq) > 0)
}

impl HistoryTable {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn get_score(&self, side: usize, mov: Move, prevs: [Move; 2], threats: u64) -> i32 {
        let entry = &self.table[side][mov.moved_pc()][mov.to()];
        let mut score = entry.score[threatened(mov.from(), threats)][threatened(mov.to(), threats)];

        for prev in prevs {
            if prev != Move::NULL {
                score += entry.continuation[prev.moved_pc() - 2][prev.to()];
            }
        }

        score
    }

    pub fn push(&mut self, mov: Move, prevs: [Move; 2], side: usize, bonus: i32, threats: u64) {
        let entry = &mut self.table[side][mov.moved_pc()][mov.to()];
        let main_entry =
            &mut entry.score[threatened(mov.from(), threats)][threatened(mov.to(), threats)];

        *main_entry += bonus - *main_entry * bonus.abs() / MoveScore::HISTORY_MAX;

        for prev in prevs {
            if prev != Move::NULL {
                let cont_entry = &mut entry.continuation[prev.moved_pc() - 2][prev.to()];
                *cont_entry += bonus - *cont_entry * bonus.abs() / MoveScore::HISTORY_MAX;
            }
        }
    }
}

pub struct NodeTable {
    table: Box<[[u64; 64]; 64]>,
}

impl Default for NodeTable {
    fn default() -> Self {
        Self {
            table: boxed_and_zeroed(),
        }
    }
}

impl NodeTable {
    pub fn get(&self, mov: Move) -> u64 {
        self.table[mov.from()][mov.to()]
    }

    pub fn update(&mut self, mov: Move, nodes: u64) {
        self.table[mov.from()][mov.to()] += nodes;
    }
}

#[derive(Clone, Copy, Default)]
pub struct PlyEntry {
    pub killer: Move,
    pub eval: i32,
    pub singular: Move,
    pub pv_line: MoveList,
    pub cutoffs: i32,
    pub dbl_exts: i32,
    pub played: Move,
}

pub struct PlyTable {
    table: Box<[PlyEntry; 128]>,
}

impl Default for PlyTable {
    fn default() -> Self {
        Self {
            table: boxed_and_zeroed(),
        }
    }
}

impl std::ops::Index<i32> for PlyTable {
    type Output = PlyEntry;
    fn index(&self, index: i32) -> &Self::Output {
        &self.table[index as usize]
    }
}

impl std::ops::IndexMut<i32> for PlyTable {
    fn index_mut(&mut self, index: i32) -> &mut Self::Output {
        &mut self.table[index as usize]
    }
}

impl PlyTable {
    pub fn clear(&mut self) {
        self.table.iter_mut().for_each(|ply| {
            ply.killer = Move::NULL;
            ply.played = Move::NULL;
        });
    }

    pub fn push_killer(&mut self, m: Move, mut ply: i32) {
        ply -= 1;
        self[ply].killer = m
    }

    pub fn prev_move(&self, ply: i32, n: i32) -> Move {
        let idx = ply + 1 - n;
        if idx >= 0 {
            self[idx].played
        } else {
            Move::NULL
        }
    }
}

const CHSIZE: usize = 16384;

#[derive(Clone)]
pub struct CorrectionHistoryTable {
    table: [[i32; CHSIZE]; 2],
}

impl CorrectionHistoryTable {
    pub fn boxed() -> Box<Self> {
        boxed_and_zeroed()
    }

    pub fn age_entries(&mut self) {
        self.table.iter_mut().flatten().for_each(|x| *x /= 2);
    }

    pub fn clear(&mut self) {
        self.table.iter_mut().for_each(|t| t.fill(0));
    }

    pub fn update_correction_history(&mut self, pos: &Position, depth: i32, diff: i32) {
        let entry = &mut self.table[pos.stm()][(pos.pawnhash() % CHSIZE as u64) as usize];
        let scaled_diff = diff * CorrectionHistory::GRAIN;
        let new_weight = 16.min(depth + 1);

        let update = *entry * (CorrectionHistory::SCALE - new_weight) + scaled_diff * new_weight;
        *entry = i32::clamp(update / CorrectionHistory::SCALE, -CorrectionHistory::MAX, CorrectionHistory::MAX);
    }

    pub fn correct_evaluation(&self, pos: &Position, raw_eval: i32) -> i32 {
        let entry = self.table[pos.stm()][(pos.pawnhash() % CHSIZE as u64) as usize];

        raw_eval + entry / CorrectionHistory::GRAIN
    }
}
