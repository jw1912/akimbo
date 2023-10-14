use std::sync::atomic::{AtomicU64, AtomicU8, Ordering::Relaxed};

use crate::{
    consts::{MoveScore, Score},
    moves::{Move, MoveList},
    position::Position,
};

pub struct HashView<'a> {
    table: &'a HashTable,
}

impl<'a> std::ops::Deref for HashView<'a> {
    type Target = HashTable;
    fn deref(&self) -> &Self::Target {
        self.table
    }
}

impl<'a> HashView<'a> {
    pub fn new(tt: &'a HashTable) -> Self {
        Self { table: tt }
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
    pub fn resize(&mut self, size: usize) {
        self.table = vec![HashEntryInternal::default(); 1 << (80 - (size as u64).leading_zeros())];
        self.age.store(0, Relaxed);
    }

    pub fn clear(&mut self) {
        self.table
            .iter_mut()
            .for_each(|entry| *entry = HashEntryInternal::default());
        self.age.store(0, Relaxed);
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

#[derive(Copy, Clone, Default)]
pub struct HistoryEntry {
    score: i32,
    counter: Move,
}

#[derive(Clone)]
pub struct HistoryTable {
    table: Box<[[[HistoryEntry; 64]; 8]; 2]>,
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self {
            table: Box::new([[[Default::default(); 64]; 8]; 2]),
        }
    }
}

impl HistoryTable {
    pub fn age(&mut self) {
        self.table.iter_mut().flatten().flatten().for_each(|entry| {
            entry.score /= 2;
            entry.counter = Move::NULL;
        });
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn get_score(&self, side: usize, mov: Move) -> i32 {
        self.table[side][mov.moved_pc()][mov.to()].score
    }

    pub fn get_counter(&self, side: usize, prev: Move) -> Move {
        self.table[side][prev.moved_pc()][prev.to()].counter
    }

    pub fn push(&mut self, mov: Move, side: usize, bonus: i32) {
        let entry = &mut self.table[side][mov.moved_pc()][mov.to()];
        entry.score += bonus - entry.score * bonus.abs() / MoveScore::HISTORY_MAX
    }

    pub fn push_counter(&mut self, side: usize, prev: Move, mov: Move) {
        self.table[side][prev.moved_pc()][prev.to()].counter = mov;
    }
}

pub struct NodeTable {
    table: Box<[[u64; 64]; 64]>,
}

impl Default for NodeTable {
    fn default() -> Self {
        Self {
            table: Box::new([[0; 64]; 64]),
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
    pub killers: [Move; 2],
    pub eval: i32,
    pub singular: Move,
    pub pv_line: MoveList,
    pub cutoffs: i32,
    pub dbl_exts: i32,
}

pub struct PlyTable {
    table: Box<[PlyEntry; 96]>,
}

impl Default for PlyTable {
    fn default() -> Self {
        Self {
            table: Box::new([Default::default(); 96]),
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
    pub fn clear_killers(&mut self) {
        self.table
            .iter_mut()
            .for_each(|ply| ply.killers = [Move::NULL; 2]);
    }

    pub fn push_killer(&mut self, m: Move, mut ply: i32) {
        ply -= 1;
        self[ply].killers[1] = self[ply].killers[0];
        self[ply].killers[0] = m;
    }
}
