use crate::{
    consts::{Piece, Rights, Side},
    position::Position,
};

#[derive(Clone, Copy)]
pub struct Castling {
    chess960: bool,
    castle_mask: [u8; 64],
    rook_files: [[u8; 2]; 2],
}

impl Default for Castling {
    fn default() -> Self {
        Self {
            chess960: false,
            castle_mask: [0; 64],
            rook_files: [[0; 2]; 2],
        }
    }
}

impl Castling {
    pub fn is_chess960(&self) -> bool {
        self.chess960
    }

    pub fn mask(&self, sq: usize) -> u8 {
        self.castle_mask[sq]
    }

    pub fn rook_file(&self, side: usize, ks: usize) -> u8 {
        self.rook_files[side][ks]
    }

    pub fn parse(&mut self, pos: &Position, rights_str: &str) -> u8 {
        let mut kings = [4, 4];

        self.chess960 = false;
        self.rook_files[0][0] = 0;
        self.rook_files[0][1] = 7;
        self.rook_files[1][0] = 0;
        self.rook_files[1][1] = 7;

        let rights = rights_str.chars().fold(0, |cr, ch| {
            cr | match ch as u8 {
                b'Q' => Rights::WQS,
                b'K' => Rights::WKS,
                b'q' => Rights::BQS,
                b'k' => Rights::BKS,
                b'A'..=b'H' => self.parse_castle(pos, Side::WHITE, &mut kings, ch),
                b'a'..=b'h' => self.parse_castle(pos, Side::BLACK, &mut kings, ch),
                _ => 0,
            }
        });

        for sq in self.castle_mask.iter_mut() {
            *sq = 15;
        }

        self.castle_mask[usize::from(self.rook_file(0, 0))] = 7;
        self.castle_mask[usize::from(self.rook_file(0, 1))] = 11;
        self.castle_mask[usize::from(self.rook_file(1, 0)) + 56] = 13;
        self.castle_mask[usize::from(self.rook_file(1, 1)) + 56] = 14;
        self.castle_mask[kings[0]] = 3;
        self.castle_mask[kings[1] + 56] = 12;

        rights
    }

    fn parse_castle(
        &mut self,
        pos: &Position,
        side: usize,
        kings: &mut [usize; 2],
        ch: char,
    ) -> u8 {
        self.chess960 = true;

        let wkc = (pos.side(side) & pos.piece(Piece::KING)).trailing_zeros() as u8 & 7;
        kings[side] = wkc as usize;
        let rook = ch as u8 - [b'A', b'a'][side];
        let i = usize::from(rook > wkc);

        self.rook_files[side][i] = rook;

        [[Rights::WQS, Rights::WKS], [Rights::BQS, Rights::BKS]][side][i]
    }
}
