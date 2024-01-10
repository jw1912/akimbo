use std::{
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::Instant,
};

use crate::{
    moves::Move,
    position::Position,
    tables::{HashTable, HashView, HistoryTable, NodeTable, PlyTable},
};

pub struct ThreadData<'a> {
    // search control
    pub timing: Instant,
    pub max_time: u128,
    pub max_nodes: u64,
    pub abort: &'a AtomicBool,

    // tables
    pub tt: HashView<'a>,
    pub htable: HistoryTable,
    pub plied: Box<PlyTable>,
    pub ntable: NodeTable,
    pub stack: Vec<u64>,

    // uci output
    pub nodes: u64,
    pub qnodes: u64,
    pub ply: i32,
    pub best_move: Move,
    pub seldepth: i32,
}

impl<'a> ThreadData<'a> {
    pub fn new(
        abort: &'a AtomicBool,
        tt: &'a HashTable,
        stack: Vec<u64>,
        htable: HistoryTable,
    ) -> Self {
        Self {
            timing: Instant::now(),
            max_time: 0,
            max_nodes: u64::MAX,
            tt: HashView::new(tt),
            htable,
            plied: Box::default(),
            ntable: NodeTable::default(),
            stack,
            nodes: 0,
            qnodes: 0,
            ply: 0,
            best_move: Move::NULL,
            seldepth: 0,
            abort,
        }
    }

    pub fn stop_is_set(&self) -> bool {
        self.abort.load(Relaxed)
    }

    pub fn store_stop(&self, val: bool) {
        self.abort.store(val, Relaxed);
    }

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

    pub fn update_accmulators(&mut self, pos: &Position) {
        self.plied[self.ply].accumulators = self.plied[self.ply - 1].accumulators;
        pos.update_accmulators(&mut self.plied[self.ply].accumulators);
    }
}
