// Structs
#[derive(Clone, Copy)]
struct Mask {
    bit: u64,
    diag: u64,
    anti: u64,
    swap: u64,
}

pub struct ZobristVals {
    pub pcs: [[[u64; 64]; 8]; 2],
    pub cr: [u64; 16],
    pub enp: [u64; 8],
    pub c: [u64; 2],
}

#[derive(Clone, Copy, Default)]
pub struct S(pub i32, pub i32);

impl std::ops::AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::SubAssign<S> for S {
    fn sub_assign(&mut self, rhs: S) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

// Macros
macro_rules! init {($i:ident, $size:expr, $($r:tt)+) => {{
    let mut $i = 0;
    let mut res = [{$($r)+}; $size];
    while $i < $size - 1 {
        $i += 1;
        res[$i] = {$($r)+};
    }
    res
}}}
macro_rules! c_enum {($t:ty, $name:ident, $($n:ident = $v:expr),*) => {
    pub struct $name;
    impl $name { $(pub const $n: $t = $v;)* }
}}

pub struct Attacks;
impl Attacks {
    pub const PAWN: [[u64; 64]; 2] = [
        init!(i, 64, (((1 << i) & !File::A) << 7) | (((1 << i) & !File::H) << 9)),
        init!(i, 64, (((1 << i) & !File::A) >> 9) | (((1 << i) & !File::H) >> 7)),
    ];

    pub const KNIGHT: [u64; 64] = init!(i, 64, {
        let n = 1 << i;
        let h1 = ((n >> 1) & 0x7f7f7f7f7f7f7f7f) | ((n << 1) & 0xfefefefefefefefe);
        let h2 = ((n >> 2) & 0x3f3f3f3f3f3f3f3f) | ((n << 2) & 0xfcfcfcfcfcfcfcfc);
        (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
    });

    pub const KING: [u64; 64] = init!(i, 64, {
        let mut k = 1 << i;
        k |= (k << 8) | (k >> 8);
        k |= ((k & !File::A) >> 1) | ((k & !File::H) << 1);
        k ^ (1 << i)
    });

    pub fn bishop(idx: usize, occ: u64) -> u64 {
        let m = MASKS[idx];
        let (mut f1, mut f2) = (occ & m.diag, occ & m.anti);
        let (r1, r2) = (f1.swap_bytes().wrapping_sub(m.swap), f2.swap_bytes().wrapping_sub(m.swap));
        f1 = f1.wrapping_sub(m.bit);
        f2 = f2.wrapping_sub(m.bit);
        ((f1 ^ r1.swap_bytes()) & m.diag) | ((f2 ^ r2.swap_bytes()) & m.anti)
    }

    pub fn rook(sq: usize, occ: u64) -> u64 {
        FILE[sq][((((occ >> (sq & 7)) & File::A).wrapping_mul(DIAG) >> 57) & 0x3F) as usize]
        | RANK[sq][((occ >> RANK_SHIFT[sq]) & 0x3F) as usize]
    }
}

// All named collections of constants
c_enum!(u8, Bound, LOWER = 0, EXACT = 1, UPPER = 2);
c_enum!(i32, Score, MAX = 30000, MATE = Self::MAX - 256, DRAW = 0);
c_enum!(i32, MoveScore, HASH = 3000000, HISTORY_MAX = 65536, PROMO = 70000, KILLER = 69000);
c_enum!(usize, Side, WHITE = 0, BLACK = 1);
c_enum!(usize, Piece, EMPTY = 0, PAWN = 2, KNIGHT = 3, BISHOP = 4, ROOK = 5, QUEEN = 6, KING = 7);
c_enum!(u8, Flag, QUIET = 0, DBL = 1, KS = 2, QS = 3, CAP = 4, ENP = 5, PROMO = 8, QPR = 11, NPC = 12, QPC = 15);
c_enum!(u8, Rights, WQS = 8, WKS = 4, BQS = 2, BKS = 1, WHITE = Self::WQS | Self::WKS, BLACK = Self::BQS | Self::BKS);
c_enum!([u64; 2], Rank, PEN = [0xFF000000000000, 0xFF00], DBL = [0xFF000000, 0xFF00000000]);
c_enum!(u64, File, A = 0x101010101010101, H = Self::A << 7);

// Movegen
const EAST: [u64; 64] = init!(i, 64, (1 << i) ^ WEST[i] ^ (0xFF << (i & 56)));
const WEST: [u64; 64] = init!(i, 64, ((1 << i) - 1) & (0xFF << (i & 56)));
const DIAG: u64 = 0x8040201008040201;
const DIAGS: [u64; 15] = init!(i, 15, if i > 7 { DIAG >> (8 * (i - 7)) } else {DIAG << (8 * (7 - i)) });
static MASKS: [Mask; 64] = init!(i, 64,
    let (bit, rank, file) = (1 << i, i / 8, i & 7);
    Mask {
        bit,
        diag: bit ^ DIAGS[7 + file - rank],
        anti: bit ^ DIAGS[file + rank].swap_bytes(),
        swap: bit.swap_bytes(),
    }
);
const RANK_SHIFT: [usize; 64] = init! {sq, 64, sq - (sq & 7) + 1};
const RANK: [[u64; 64]; 64] = init!(sq, 64, init!(i, 64, {
    let (f, occ) = (sq & 7, (i << 1) as u64);
    (EAST[f] ^ EAST[( (EAST[f] & occ) | (1<<63)).trailing_zeros() as usize]
    | WEST[f] ^ WEST[(((WEST[f] & occ) | 1).leading_zeros() ^ 63) as usize]) << (sq - f)
}));
const FILE: [[u64; 64]; 64] = init! {sq, 64, init! {occ, 64, (RANK[7 - sq / 8][occ].wrapping_mul(DIAG) & File::H) >> (7 - (sq & 7))}};

pub const CASTLE_MASK: [u8; 64] = init! {idx, 64, match idx {0 => 7, 4 => 3, 7 => 11, 56 => 13, 60 => 12, 63 => 14, _ => 15}};
pub const ROOK_MOVES: [[(u64, usize, usize); 2]; 2] = [[(9, 0, 3), (0x0900000000000000, 56, 59)], [(160, 7, 5), (0xA000000000000000, 63, 61)]];

// Zobrist values
const fn rand(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = rand(seed);
    let c = [0, seed];
    let pcs = init!(side, 2, init!(pc, 8, init!(sq, 64, {
        if pc < 2 { 0 } else { seed = rand(seed); seed }
    })));
    let cf = init!(i, 4, {seed = rand(seed); seed});
    let cr = init!(i, 16, {
          ((i & 1 > 0) as u64 * cf[0]) ^ ((i & 2 > 0) as u64 * cf[1])
        ^ ((i & 4 > 0) as u64 * cf[2]) ^ ((i & 8 > 0) as u64 * cf[3])
    });
    let enp = init!(i, 8, {seed = rand(seed); seed});
    ZobristVals { pcs, cr, enp, c }
};

// Eval
pub const SIDE: [i32; 2] = [1, -1];
pub const PHASE_VALS: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];
pub static PST: [[[S; 64]; 8]; 2] = [
    init!(i, 8, init!(j, 64, RAW_PST[i][j ^ 56])),
    init!(i, 8, init!(j, 64, S(-RAW_PST[i][j].0, -RAW_PST[i][j].1))),
];
const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
    [
        S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100),
        S( 227, 258), S( 239, 256), S( 193, 238), S( 210, 221), S( 204, 228), S( 221, 223), S( 148, 251), S( 143, 264),
        S(  66, 201), S(  85, 203), S( 110, 184), S( 117, 164), S( 156, 147), S( 152, 144), S( 103, 183), S(  57, 188),
        S(  57, 135), S(  94, 121), S(  89, 110), S( 106,  97), S( 108,  90), S(  96,  99), S( 103, 110), S(  55, 116),
        S(  44, 116), S(  77, 107), S(  76,  94), S(  95,  86), S( 100,  85), S(  90,  85), S(  94,  97), S(  52,  98),
        S(  46, 106), S(  75, 106), S(  76,  89), S(  69, 104), S(  85,  98), S(  87,  90), S( 117,  91), S(  66,  89),
        S(  36, 119), S(  78, 108), S(  59, 108), S(  55, 117), S(  64, 113), S( 107,  95), S( 121,  96), S(  55,  92),
        S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100), S( 100, 100),
    ], [
        S( 160, 221), S( 232, 248), S( 279, 278), S( 281, 253), S( 396, 246), S( 230, 258), S( 280, 228), S( 207, 193),
        S( 250, 261), S( 285, 277), S( 407, 252), S( 364, 278), S( 358, 270), S( 402, 250), S( 334, 258), S( 318, 229),
        S( 284, 258), S( 398, 256), S( 372, 290), S( 401, 286), S( 429, 269), S( 455, 269), S( 411, 256), S( 382, 233),
        S( 322, 266), S( 352, 283), S( 354, 303), S( 390, 301), S( 374, 300), S( 408, 289), S( 355, 285), S( 357, 262),
        S( 320, 265), S( 340, 274), S( 349, 297), S( 347, 307), S( 362, 297), S( 355, 296), S( 355, 285), S( 326, 261),
        S( 310, 259), S( 324, 281), S( 347, 275), S( 346, 293), S( 355, 290), S( 354, 271), S( 359, 261), S( 318, 254),
        S( 302, 242), S( 273, 266), S( 321, 271), S( 330, 277), S( 332, 282), S( 354, 255), S( 316, 259), S( 316, 238),
        S( 213, 256), S( 311, 238), S( 267, 265), S( 295, 267), S( 316, 259), S( 304, 264), S( 315, 245), S( 305, 219),
    ], [
        S( 326, 285), S( 361, 276), S( 243, 299), S( 294, 295), S( 304, 299), S( 311, 290), S( 342, 287), S( 348, 274),
        S( 336, 289), S( 380, 289), S( 344, 304), S( 337, 289), S( 395, 291), S( 416, 282), S( 382, 291), S( 315, 285),
        S( 346, 298), S( 398, 286), S( 406, 294), S( 403, 293), S( 396, 292), S( 417, 299), S( 396, 296), S( 363, 299),
        S( 358, 295), S( 369, 306), S( 382, 308), S( 413, 304), S( 403, 308), S( 402, 304), S( 370, 299), S( 359, 301),
        S( 355, 291), S( 377, 298), S( 375, 309), S( 387, 314), S( 397, 302), S( 375, 302), S( 373, 292), S( 365, 288),
        S( 362, 284), S( 378, 294), S( 376, 305), S( 378, 304), S( 375, 309), S( 391, 296), S( 380, 289), S( 371, 280),
        S( 366, 283), S( 380, 273), S( 378, 289), S( 361, 300), S( 371, 300), S( 382, 287), S( 397, 273), S( 366, 267),
        S( 323, 276), S( 358, 289), S( 347, 283), S( 337, 293), S( 346, 290), S( 349, 290), S( 322, 291), S( 337, 280),
    ], [
        S( 507, 529), S( 526, 522), S( 497, 536), S( 550, 521), S( 545, 522), S( 469, 534), S( 486, 528), S( 504, 523),
        S( 502, 526), S( 503, 529), S( 544, 521), S( 547, 520), S( 570, 501), S( 559, 510), S( 492, 526), S( 514, 518),
        S( 455, 530), S( 484, 527), S( 495, 524), S( 499, 525), S( 479, 524), S( 524, 509), S( 541, 506), S( 480, 516),
        S( 435, 530), S( 450, 527), S( 469, 535), S( 492, 520), S( 484, 523), S( 502, 518), S( 462, 519), S( 442, 527),
        S( 420, 530), S( 433, 530), S( 452, 531), S( 460, 526), S( 467, 518), S( 454, 517), S( 475, 510), S( 440, 512),
        S( 413, 523), S( 438, 524), S( 448, 517), S( 443, 523), S( 464, 513), S( 460, 510), S( 462, 511), S( 431, 506),
        S( 414, 519), S( 448, 516), S( 442, 524), S( 451, 526), S( 461, 513), S( 471, 513), S( 457, 510), S( 394, 520),
        S( 441, 519), S( 447, 527), S( 462, 527), S( 473, 523), S( 476, 517), S( 460, 515), S( 432, 525), S( 441, 503),
    ], [
        S( 907, 962), S( 933, 994), S( 954, 995), S( 954, 993), S(1017, 977), S(1004, 969), S( 981, 968), S( 977, 989),
        S( 923, 944), S( 903, 982), S( 938, 998), S( 946,1005), S( 919,1031), S(1003, 982), S( 971, 988), S( 994, 956),
        S( 935, 937), S( 926, 970), S( 953, 967), S( 945,1017), S( 975,1008), S(1004, 989), S( 992, 976), S( 999, 969),
        S( 909, 976), S( 912, 994), S( 925, 991), S( 925,1014), S( 938,1028), S( 952,1014), S( 937,1030), S( 936,1013),
        S( 933, 937), S( 912, 998), S( 931, 985), S( 927,1020), S( 936,1001), S( 938,1000), S( 941,1010), S( 940, 982),
        S( 921, 962), S( 945, 934), S( 929, 983), S( 939, 963), S( 935, 977), S( 942, 983), S( 954, 975), S( 942, 979),
        S( 906, 941), S( 933, 940), S( 954, 932), S( 943, 944), S( 951, 939), S( 958, 933), S( 939, 927), S( 944, 927),
        S( 939, 930), S( 924, 934), S( 932, 938), S( 950, 940), S( 924, 960), S( 910, 936), S( 902, 953), S( 894, 914),
    ], [
        S( -21, -63), S(  85, -35), S(  81, -22), S(  52, -21), S( -55,   5), S( -31,  26), S(  34,  10), S(  23,  -5),
        S(  96, -18), S(  47,  17), S(  37,  13), S(  88,   7), S(  41,  16), S(  29,  41), S( -21,  35), S( -75,  32),
        S(  50,   8), S(  66,  18), S(  75,  19), S(  30,  17), S(  43,  17), S(  90,  38), S(  94,  38), S(  -8,  21),
        S(   4,  -4), S(  -6,  29), S(  18,  29), S( -29,  39), S( -24,  36), S( -28,  45), S(   0,  34), S( -68,  21),
        S( -63,  -6), S(  24,  -1), S( -48,  36), S( -95,  47), S( -97,  50), S( -64,  41), S( -55,  26), S( -73,   6),
        S(  15, -18), S(  -3,   4), S( -32,  23), S( -68,  38), S( -66,  41), S( -56,  34), S(  -9,  16), S( -28,   2),
        S(  21, -28), S(  22,  -9), S( -12,  15), S( -73,  28), S( -54,  28), S( -22,  17), S(  23,  -2), S(  31, -20),
        S(  -4, -53), S(  55, -43), S(  23, -21), S( -66,   3), S(  10, -23), S( -32,  -3), S(  43, -30), S(  40, -54),
    ],
];
