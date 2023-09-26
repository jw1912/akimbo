use crate::{
    consts::File,
    init,
};

pub struct Attacks;
impl Attacks {
    pub fn pawn(side: usize, sq: usize) -> u64 {
        PAWN[side][sq]
    }

    pub fn knight(sq: usize) -> u64 {
        KNIGHT[sq]
    }

    pub fn bishop(sq: usize, occ: u64) -> u64 {
        let mask = MASKS[sq];

        let mut diag = occ & mask.diag;
        let mut rev1 = diag.swap_bytes();
        diag = diag.wrapping_sub(mask.bit);
        rev1 = rev1.wrapping_sub(mask.swap);
        diag ^= rev1.swap_bytes();
        diag &= mask.diag;

        let mut anti = occ & mask.anti;
        let mut rev2 = anti.swap_bytes();
        anti = anti.wrapping_sub(mask.bit);
        rev2 = rev2.wrapping_sub(mask.swap);
        anti ^= rev2.swap_bytes();
        anti &= mask.anti;

        diag | anti
    }

    pub fn rook(sq: usize, occ: u64) -> u64 {
        let flip = ((occ >> (sq & 7)) & File::A).wrapping_mul(DIAG);
        let file_sq = (flip >> 57) & 0x3F;
        let files = FILE[sq][file_sq as usize];

        let rank_sq = (occ >> RANK_SHIFT[sq]) & 0x3F;
        let ranks = RANK[sq][rank_sq as usize];

        ranks | files
    }

    pub fn queen(sq: usize, occ: u64) -> u64 {
        Self::bishop(sq, occ) | Self::rook(sq, occ)
    }

    pub fn king(sq: usize) -> u64 {
        KING[sq]
    }
}

#[derive(Clone, Copy)]
struct Mask {
    bit: u64,
    diag: u64,
    anti: u64,
    swap: u64,
}

const PAWN: [[u64; 64]; 2] = [
        init!(|sq, 64| {
            let bit = 1 << sq;
            ((bit & !File::A) << 7) | ((bit & !File::H) << 9)
        }),
        init!(|sq, 64| {
            let bit = 1 << sq;
            ((bit & !File::A) >> 9) | ((bit & !File::H) >> 7)
        }),
    ];

const KNIGHT: [u64; 64] = init!(|sq, 64| {
    let n = 1 << sq;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});

const KING: [u64; 64] = init!(|sq, 64| {
    let mut k = 1 << sq;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !File::A) >> 1) | ((k & !File::H) << 1);
    k ^ (1 << sq)
});

const EAST: [u64; 64] = init!(|sq, 64| (1 << sq) ^ WEST[sq] ^ (0xFF << (sq & 56)));

const WEST: [u64; 64] = init!(|sq, 64| ((1 << sq) - 1) & (0xFF << (sq & 56)));

const DIAG: u64 = 0x8040201008040201;

const DIAGS: [u64; 15] = init!(|sq, 15|
    if sq > 7 {
        DIAG >> (8 * (sq - 7))
    } else {
        DIAG << (8 * (7 - sq))
    }
);

static MASKS: [Mask; 64] = init!(|sq, 64|
    let bit = 1 << sq;
    let file = sq & 7;
    let rank = sq / 8;
    Mask {
        bit,
        diag: bit ^ DIAGS[7 + file - rank],
        anti: bit ^ DIAGS[file + rank].swap_bytes(),
        swap: bit.swap_bytes(),
    }
);

const RANK_SHIFT: [usize; 64] = init!(|sq, 64| sq - (sq & 7) + 1);

const RANK: [[u64; 64]; 64] = init!(|sq, 64|
    init!(|i, 64| {
        let f = sq & 7;
        let occ = (i << 1) as u64;
        let east = EAST[f] ^ EAST[((EAST[f] & occ) | (1<<63)).trailing_zeros() as usize];
        let west = WEST[f] ^ WEST[(((WEST[f] & occ) | 1).leading_zeros() ^ 63) as usize];
        (east | west) << (sq - f)
    })
);

const FILE: [[u64; 64]; 64] = init!(|sq, 64|
    init!(|occ, 64|
        (RANK[7 - sq / 8][occ].wrapping_mul(DIAG) & File::H) >> (7 - (sq & 7))
    )
);