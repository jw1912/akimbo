use super::{consts::*, decl, decl_mut, position::{Position, Move, ratt, batt}};
use std::mem::MaybeUninit;

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    while $bb > 0 {
        let $sq = $bb.trailing_zeros() as u8;
        $bb &= $bb - 1;
        $func;
    }
}}

pub struct List<T> {
    pub list: [T; 252],
    pub len: usize,
}

pub type MoveList = List<Move>;
pub type ScoreList = List<i16>;

impl<T> List<T> {
    pub fn uninit() -> Self {
        #[allow(clippy::uninit_assumed_init, invalid_value)]
        Self { list: unsafe { MaybeUninit::uninit().assume_init() }, len: 0 }
    }

    #[inline(always)]
    pub fn add(&mut self, entry: T) {
        self.list[self.len] = entry;
        self.len += 1;
    }
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, from: u8, to: u8, flag: u8, mpc: u8) {
        self.list[self.len] = Move { from, to, flag, mpc };
        self.len += 1;
    }

    pub fn pick(&mut self, scores: &mut ScoreList) -> Option<(Move, i16)> {
        if scores.len == 0 { return None }
        decl_mut!(idx = 0, best = i16::MIN);
        for i in 0..scores.len {
            let score = scores.list[i];
            if score > best {
                best = score;
                idx = i;
            }
        }
        scores.len -= 1;
        scores.list.swap(idx, scores.len);
        self.list.swap(idx, scores.len);
        Some((self.list[scores.len], best))
    }
}

#[inline(always)]
fn encode<const PC: usize, const FLAG: u8>(moves: &mut MoveList, mut attacks: u64, from: u8) {
    bitloop!(attacks, to, moves.push(from, to, FLAG, PC as u8))
}

impl Position {
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::uninit();
        decl!(side = usize::from(self.c), occ = self.bb[0] | self.bb[1]);
        decl!(friends = self.bb[side], opps = self.bb[side ^ 1], pawns = self.bb[P] & friends);
        if QUIETS {
            if self.state.cr & CS[side] > 0 && !self.in_check() {self.castles(&mut moves, occ)}
            if side == WH {pushes::<WH>(&mut moves, occ, pawns)} else {pushes::<BL>(&mut moves, occ, pawns)}
        }
        if self.state.enp > 0 {en_passants(&mut moves, pawns, self.state.enp, side)}
        pawn_caps(&mut moves, pawns, opps, side);
        pc_moves::<N, QUIETS>(&mut moves, occ, friends, opps, self.bb[N]);
        pc_moves::<B, QUIETS>(&mut moves, occ, friends, opps, self.bb[B]);
        pc_moves::<R, QUIETS>(&mut moves, occ, friends, opps, self.bb[R]);
        pc_moves::<Q, QUIETS>(&mut moves, occ, friends, opps, self.bb[Q]);
        pc_moves::<K, QUIETS>(&mut moves, occ, friends, opps, self.bb[K]);
        moves
    }

    fn castles(&self, moves: &mut MoveList, occ: u64) {
        if self.c {
            if self.state.cr & BQS > 0 && occ & BD8 == 0 && !self.is_sq_att(59, BL, occ) {moves.push(60, 58, QS, K as u8)}
            if self.state.cr & BKS > 0 && occ & FG8 == 0 && !self.is_sq_att(61, BL, occ) {moves.push(60, 62, KS, K as u8)}
        } else {
            if self.state.cr & WQS > 0 && occ & BD1 == 0 && !self.is_sq_att(3, WH, occ) {moves.push(4, 2, QS, K as u8)}
            if self.state.cr & WKS > 0 && occ & FG1 == 0 && !self.is_sq_att(5, WH, occ) {moves.push(4, 6, KS, K as u8)}
        }
    }
}

fn pc_moves<const PC: usize, const QUIETS: bool>(moves: &mut MoveList, occ: u64, friends: u64, opps: u64, mut attackers: u64) {
    attackers &= friends;
    bitloop!(attackers, from, {
        let f = from as usize;
        let attacks = match PC {
            N => NATT[f],
            R => ratt(f, occ),
            B => batt(f, occ),
            Q => ratt(f, occ) | batt(f, occ),
            K => KATT[f],
            _ => 0
        };
        encode::<PC, CAP>(moves, attacks & opps, from);
        if QUIETS { encode::<PC, QUIET>(moves, attacks & !occ, from) }
    });
}

fn pawn_caps(moves: &mut MoveList, mut attackers: u64, opps: u64, c: usize) {
    let mut promo = attackers & PENRANK[c];
    attackers &= !PENRANK[c];
    bitloop!(attackers, from, encode::<P, CAP>(moves, PATT[c][from as usize] & opps, from));
    bitloop!(promo, from, {
        let mut attacks = PATT[c][from as usize] & opps;
        bitloop!(attacks, to, for flag in NPC..=QPC { moves.push(from, to, flag, P as u8) })
    });
}

fn en_passants(moves: &mut MoveList, pawns: u64, sq: u8, c: usize) {
    let mut attackers = PATT[c ^ 1][sq as usize] & pawns;
    bitloop!(attackers, from, moves.push(from, sq, ENP, P as u8))
}

fn shift<const SIDE: usize>(bb: u64) -> u64 {
    if SIDE == WH {bb >> 8} else {bb << 8}
}

fn idx_shift<const SIDE: usize, const AMOUNT: u8>(idx: u8) -> u8 {
    if SIDE == WH {idx + AMOUNT} else {idx - AMOUNT}
}

fn pushes<const SIDE: usize>(moves: &mut MoveList, occ: u64, pawns: u64) {
    let empty = !occ;
    let mut dbl = shift::<SIDE>(shift::<SIDE>(empty & DBLRANK[SIDE]) & empty) & pawns;
    decl_mut!(push = shift::<SIDE>(empty) & pawns, promo = push & PENRANK[SIDE]);
    push &= !PENRANK[SIDE];
    bitloop!(push, from, moves.push(from, idx_shift::<SIDE, 8>(from), QUIET, P as u8));
    bitloop!(promo, from, for flag in NPR..=QPR {moves.push(from, idx_shift::<SIDE, 8>(from), flag, P as u8)});
    bitloop!(dbl, from, moves.push(from, idx_shift::<SIDE, 16>(from), DBL, P as u8));
}
