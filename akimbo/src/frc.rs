use crate::{consts::{Piece, Rights, Side}, position::Position};

use std::sync::atomic::{AtomicU8, AtomicBool, Ordering::Relaxed};

#[allow(clippy::declare_interior_mutable_const)]
const ATOMIC_INIT: AtomicU8 = AtomicU8::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const ATOMIC_INIT_2: [AtomicU8; 2] = [ATOMIC_INIT; 2];
static CHESS960: AtomicBool = AtomicBool::new(false);
static CASTLE_MASK: [AtomicU8; 64] = [ATOMIC_INIT; 64];
static ROOK_FILES: [[AtomicU8; 2]; 2] = [ATOMIC_INIT_2; 2];

pub struct Castling;
impl Castling {
    pub fn is_chess960() -> bool {
        CHESS960.load(Relaxed)
    }

    pub fn mask(sq: usize) -> u8 {
        CASTLE_MASK[sq].load(Relaxed)
    }

    pub fn rook_file(side: usize, ks: usize) -> u8 {
        ROOK_FILES[side][ks].load(Relaxed)
    }

    pub fn parse(pos: &Position, rights_str: &str) -> u8 {
        let mut kings = [4, 4];

        CHESS960.store(false, Relaxed);
        ROOK_FILES[0][0].store(0, Relaxed);
        ROOK_FILES[0][1].store(7, Relaxed);
        ROOK_FILES[1][0].store(0, Relaxed);
        ROOK_FILES[1][1].store(7, Relaxed);

        let rights = rights_str
            .chars()
            .fold(0, |cr, ch| cr | match ch as u8 {
                b'Q' => Rights::WQS,
                b'K' => Rights::WKS,
                b'q' => Rights::BQS,
                b'k' => Rights::BKS,
                b'A'..=b'H' => parse_castle(pos, Side::WHITE, &mut kings, ch),
                b'a'..=b'h' => parse_castle(pos, Side::BLACK, &mut kings, ch),
                _ => 0
            });

        for sq in &CASTLE_MASK {
            sq.store(15, Relaxed);
        }

        CASTLE_MASK[usize::from(Self::rook_file(0, 0))].store(7, Relaxed);
        CASTLE_MASK[usize::from(Self::rook_file(0, 1))].store(11, Relaxed);
        CASTLE_MASK[usize::from(Self::rook_file(1, 0)) + 56].store(13, Relaxed);
        CASTLE_MASK[usize::from(Self::rook_file(1, 1)) + 56].store(14, Relaxed);
        CASTLE_MASK[kings[0]].store( 3, Relaxed);
        CASTLE_MASK[kings[1] + 56].store(12, Relaxed);

        rights
    }
}

fn parse_castle(pos: &Position, side: usize, kings: &mut [usize; 2], ch: char) -> u8 {
    CHESS960.store(true, Relaxed);

    let wkc = (pos.side(side) & pos.piece(Piece::KING)).trailing_zeros() as u8 & 7;
    kings[side] = wkc as usize;
    let rook = ch as u8 - [b'A', b'a'][side];
    let i = usize::from(rook > wkc);

    ROOK_FILES[side][i].store(rook, Relaxed);

    [[Rights::WQS, Rights::WKS], [Rights::BQS, Rights::BKS]][side][i]
}
