use crate::{
    consts::{MoveScore, Score},
    moves::{Move, MoveList},
    position::Position,
};

#[derive(Clone, Copy, Default)]
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
}

#[derive(Default)]
pub struct HashTable {
    table: Vec<HashEntry>,
    age: u8,
}

impl HashTable {
    pub fn resize(&mut self, size: usize) {
        self.table = vec![HashEntry::default(); 1 << (80 - (size as u64).leading_zeros())];
        self.age = 0;
    }

    pub fn clear(&mut self) {
        self.table.iter_mut().for_each(|entry| *entry = HashEntry::default());
        self.age = 0;
    }

    pub fn age(&mut self) {
        self.age = 63.min(self.age + 1);
    }

    pub fn push(
        &mut self,
        hash: u64,
        mov: Move,
        depth: i8,
        bound: u8,
        mut score: i32,
        ply: i32,
    ) {

        let key = (hash >> 48) as u16;
        let idx = (hash as usize) & (self.table.len() - 1);
        let entry = &mut self.table[idx];

        // replacement scheme
        let diff = self.age - (entry.bound >> 2);
        if ply > 0
            && key == entry.key
            && depth as u8 + 2 * diff < entry.depth as u8
        {
            return;
        }

        // replace entry
        score += if score.abs() > Score::MATE {score.signum() * ply} else {0};
        let best_move = mov.to_short();
        *entry = HashEntry {
            key,
            best_move,
            score: score as i16,
            depth,
            bound: (self.age << 2) | bound,
        };
    }

    pub fn probe(&self, hash: u64, ply: i32) -> Option<HashEntry> {
        let mut entry = self.table[(hash as usize) & (self.table.len() - 1)];
        if entry.key != (hash >> 48) as u16 { return None }
        entry.score -= if entry.score.abs() > Score::MATE as i16 {entry.score.signum() * ply as i16} else {0};
        Some(entry)
    }
}

#[derive(Copy, Clone, Default)]
pub struct HistoryEntry {
    score: i32,
    counter: Move,
}

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
        self.table
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(|entry| {
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

