// Structs
#[derive(Clone, Copy)]
pub struct Mask {
    pub bit: u64,
    pub diag: u64,
    pub anti: u64,
    pub file: u64,
}

// Macros
macro_rules! consts {{$t:ty, $($n:ident = $v:expr),*} => {$(pub const $n: $t = $v;)*}}
macro_rules! init {($i:ident, $size:expr, $($r:tt)+) => {{
    let mut $i = 0;
    let mut res = [{$($r)+}; $size];
    while $i < $size {
        res[$i] = {$($r)+};
        $i += 1;
    }
    res
}}}

// UCI
consts!(&str, NAME = env!("CARGO_PKG_NAME"), VERSION = env!("CARGO_PKG_VERSION"), AUTHOR = env!("CARGO_PKG_AUTHORS"));
pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// Search
pub const KILLERS: usize = 2;
pub const HISTORY_MAX: i64 = 2048;
consts!(u8, LOWER = 1, EXACT = 2, UPPER = 3);
consts!(i16, MAX_PLY = 96, MAX = 30000, MATE = MAX - 256, HASH = MAX, MVV_LVA = 2048, PROMOTION = 3000, KILLER = 2500);

// Pieces, sides, movegen type
consts!(bool, ALL = true, CAPTURES = false);
consts!(usize, E = 0, WH = 0, BL = 1, P = 2, N = 3, B = 4, R = 5, Q = 6, K = 7);
consts!(u8, QUIET = 0, DBL = 1, KS = 2, QS = 3, CAP = 4, ENP = 5, NPR = 8, QPR = 11, NPC = 12, QPC = 15);

// Castling
consts!(u8, WQS = 8, WKS = 4, BQS = 2, BKS = 1);
consts!(u64, BD1 = 0xE, FG1 = 0x60, BD8 = 0xE00000000000000, FG8 = 0x6000000000000000);
pub const CS: [u8; 2] = [WKS | WQS, BKS | BQS];
pub const CR: [u8; 64] = init!(i, 64, match i {0 => 7, 4 => 3, 7 => 11, 56 => 13, 60 => 12, 63 => 14, _ => 15});
pub const CM: [[(u64, usize, usize); 2]; 2] = [[(9, 0, 3), (0x900000000000000, 56, 59)], [(160, 7, 5), (0xA000000000000000, 63, 61)]];

// Pawns
consts!([u64; 2], PENRANK = [0xFF000000000000, 0xFF00], DBLRANK = [0xFF000000, 0xFF00000000]);
consts!(u64, FILE = 0x101010101010101, NOTH = !(FILE << 7));
pub static PATT: [[u64; 64]; 2] = [
    init!(i, 64, (((1 << i) & !FILE) << 7) | (((1 << i) & NOTH) << 9)),
    init!(i, 64, (((1 << i) & !FILE) >> 9) | (((1 << i) & NOTH) >> 7)),
];

// King and knight attacks
pub static NATT: [u64; 64] = init!(i, 64, {
    let n = 1 << i;
    let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
    let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
    (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
});
pub static KATT: [u64; 64] = init!(i, 64, {
    let mut k = 1 << i;
    k |= (k << 8) | (k >> 8);
    k |= ((k & !FILE) >> 1) | ((k & NOTH) << 1);
    k ^ (1 << i)
});

// Slider attacks
const EA: [u64; 64] = init!(i, 64, (1 << i) ^ WE[i] ^ (0xFF << (i & 56)));
const WE: [u64; 64] = init!(i, 64, ((1 << i) - 1) & (0xFF << (i & 56)));
pub const DIAGS: [u64; 15] = [
    0x0100000000000000, 0x0201000000000000, 0x0402010000000000, 0x0804020100000000, 0x1008040201000000,
    0x2010080402010000, 0x4020100804020100, 0x8040201008040201, 0x0080402010080402, 0x0000804020100804,
    0x0000008040201008, 0x0000000080402010, 0x0000000000804020, 0x0000000000008040, 0x0000000000000080,
];
pub static MASKS: [Mask; 64] = init!(i, 64,
    let bit = 1 << i;
    Mask { bit, diag: bit ^ DIAGS[7 + (i & 7) - i / 8], anti: bit ^ DIAGS[(i & 7) + i / 8].swap_bytes(), file: bit ^ FILE << (i & 7) }
);
pub const RANKS: [[u64; 64]; 8] = init!(f, 8, init!(i, 64, {
    let occ = (i << 1) as u64;
    EA[f] ^ EA[((EA[f] & occ) | (1 << 63)).trailing_zeros() as usize] | WE[f] ^ WE[(((WE[f] & occ) | 1).leading_zeros() ^ 63) as usize]
}));

// Draw detection
consts!(u64, LSQ = 0x55AA55AA55AA55AA, DSQ = 0xAA55AA55AA55AA55);
