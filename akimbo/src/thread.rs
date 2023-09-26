use std::time::Instant;

use crate::{
    moves::Move,
    position::Position,
    tables::{HashTable, HistoryTable, NodeTable, PlyTable},
};

pub struct ThreadData {
    // search control
    pub timing: Instant,
    pub max_time: u128,
    pub max_nodes: u64,
    pub abort: bool,
    pub mloop: bool,

    // tables
    pub tt: HashTable,
    pub htable: HistoryTable,
    pub plied: PlyTable,
    pub ntable: NodeTable,
    pub stack: Vec<u64>,

    // uci output
    pub nodes: u64,
    pub qnodes: u64,
    pub ply: i32,
    pub best_move: Move,
    pub seldepth: i32,
}

impl Default for ThreadData {
    fn default() -> Self {
        Self {
            timing: Instant::now(),
            max_time: 0,
            abort: false,
            max_nodes: u64::MAX,
            mloop: true,
            tt: HashTable::default(),
            htable: HistoryTable::default(),
            plied: PlyTable::default(),
            ntable: NodeTable::default(),
            stack: Vec::with_capacity(96),
            nodes: 0,
            qnodes: 0,
            ply: 0,
            best_move: Move::NULL,
            seldepth: 0,
        }
    }
}

impl ThreadData {
    pub fn repetition(&self, pos: &Position, curr_hash: u64, root: bool) -> bool {
        if self.stack.len() < 6 {
            return false;
        }
        let mut reps = 1 + u8::from(root);
        for &hash in self
            .stack
            .iter()
            .rev()
            .take(pos.halfm() + 1)
            .skip(1)
            .step_by(2)
        {
            reps -= u8::from(hash == curr_hash);
            if reps == 0 {
                return true;
            }
        }
        false
    }

    pub fn push(&mut self, hash: u64) {
        self.ply += 1;
        self.stack.push(hash);
        self.plied[self.ply].cutoffs = 0;
    }

    pub fn pop(&mut self) {
        self.stack.pop();
        self.ply -= 1;
    }
}
