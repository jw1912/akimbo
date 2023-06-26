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
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S( 100, 100), S( 100, 100), S(100, 100), S(100, 100),
        S(225, 254), S(236, 251), S(192, 237), S(208, 219), S( 202, 226), S( 217, 220), S(149, 249), S(148, 260),
        S( 67, 198), S( 85, 202), S(109, 183), S(115, 164), S( 155, 146), S( 149, 143), S(103, 181), S( 57, 186),
        S( 57, 134), S( 92, 121), S( 88, 109), S(104,  98), S( 107,  91), S(  95,  98), S(101, 111), S( 55, 115),
        S( 43, 116), S( 77, 106), S( 76,  93), S( 94,  86), S( 100,  86), S(  89,  85), S( 94,  96), S( 52,  96),
        S( 46, 105), S( 75, 104), S( 76,  89), S( 71,  98), S(  85,  95), S(  87,  89), S(117,  89), S( 66,  88),
        S( 36, 117), S( 78, 105), S( 59, 106), S( 56, 106), S(  65, 108), S( 107,  93), S(121,  93), S( 56,  90),
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S( 100, 100), S( 100, 100), S(100, 100), S(100, 100),
    ], [
        S(158, 224), S(231, 243), S(277, 272), S(282, 247), S( 395, 238), S( 232, 251), S(277, 220), S(204, 190),
        S(250, 255), S(285, 271), S(408, 245), S(365, 272), S( 359, 263), S( 401, 244), S(334, 252), S(317, 223),
        S(284, 252), S(398, 250), S(372, 284), S(401, 280), S( 429, 263), S( 452, 262), S(412, 249), S(382, 227),
        S(323, 259), S(353, 277), S(354, 297), S(391, 293), S( 375, 294), S( 409, 281), S(355, 279), S(358, 254),
        S(321, 256), S(340, 268), S(349, 291), S(348, 300), S( 363, 289), S( 356, 291), S(355, 279), S(328, 253),
        S(312, 250), S(325, 272), S(348, 270), S(346, 287), S( 355, 283), S( 354, 268), S(361, 251), S(319, 250),
        S(303, 235), S(274, 259), S(322, 264), S(332, 267), S( 334, 271), S( 354, 251), S(318, 252), S(318, 227),
        S(214, 249), S(315, 217), S(269, 256), S(296, 260), S( 318, 251), S( 305, 255), S(318, 220), S(307, 210),
    ], [
        S(328, 277), S(360, 270), S(246, 292), S(293, 290), S( 308, 291), S( 312, 285), S(345, 280), S(350, 266),
        S(337, 282), S(379, 284), S(344, 298), S(337, 283), S( 396, 284), S( 416, 276), S(382, 284), S(315, 279),
        S(347, 291), S(398, 280), S(405, 288), S(403, 286), S( 397, 286), S( 418, 291), S(395, 290), S(364, 293),
        S(359, 287), S(370, 299), S(382, 301), S(413, 297), S( 403, 302), S( 403, 297), S(371, 292), S(360, 292),
        S(356, 284), S(379, 290), S(376, 303), S(387, 308), S( 397, 296), S( 376, 299), S(373, 286), S(366, 282),
        S(363, 278), S(379, 286), S(377, 298), S(379, 299), S( 376, 303), S( 391, 290), S(380, 284), S(371, 275),
        S(368, 275), S(380, 270), S(378, 283), S(363, 289), S( 371, 293), S( 383, 280), S(397, 272), S(367, 261),
        S(324, 269), S(359, 281), S(350, 264), S(338, 286), S( 347, 282), S( 351, 271), S(319, 287), S(336, 276),
    ], [
        S(503, 522), S(521, 515), S(495, 529), S(545, 514), S( 541, 515), S( 465, 527), S(482, 521), S(499, 516),
        S(499, 518), S(500, 521), S(541, 513), S(545, 513), S( 566, 495), S( 556, 502), S(489, 519), S(510, 512),
        S(452, 523), S(482, 520), S(492, 517), S(496, 518), S( 475, 518), S( 521, 502), S(538, 499), S(478, 509),
        S(433, 522), S(449, 519), S(467, 527), S(490, 513), S( 482, 515), S( 499, 510), S(459, 511), S(439, 520),
        S(418, 522), S(431, 523), S(449, 523), S(459, 519), S( 465, 511), S( 452, 510), S(474, 502), S(437, 506),
        S(411, 516), S(436, 517), S(445, 510), S(440, 516), S( 461, 506), S( 458, 502), S(459, 504), S(428, 501),
        S(411, 514), S(446, 509), S(439, 516), S(449, 518), S( 459, 506), S( 469, 505), S(456, 502), S(389, 519),
        S(440, 509), S(446, 519), S(460, 519), S(470, 516), S( 472, 511), S( 458, 508), S(429, 518), S(440, 491),
    ], [
        S(899, 947), S(922, 980), S(948, 978), S(943, 980), S(1018, 952), S(1006, 945), S(981, 945), S(974, 966),
        S(913, 930), S(891, 974), S(929, 983), S(933, 994), S( 910,1016), S( 997, 963), S(962, 972), S(986, 940),
        S(924, 926), S(917, 956), S(943, 953), S(935,1005), S( 961, 998), S(1000, 968), S(982, 962), S(993, 948),
        S(899, 964), S(903, 978), S(916, 976), S(917, 999), S( 929,1013), S( 946, 993), S(928,1015), S(930, 992),
        S(924, 928), S(901, 988), S(922, 974), S(921,1003), S( 927, 988), S( 929, 985), S(934, 990), S(930, 972),
        S(913, 944), S(937, 914), S(920, 968), S(930, 956), S( 926, 963), S( 933, 969), S(946, 960), S(935, 958),
        S(896, 929), S(923, 928), S(946, 913), S(934, 933), S( 941, 934), S( 948, 922), S(933, 906), S(937, 908),
        S(930, 915), S(916, 916), S(924, 925), S(945, 892), S( 915, 946), S( 901, 921), S(898, 930), S(883, 902),
    ], [
        S(-25, -65), S( 84, -41), S( 83, -28), S( 53, -25), S( -56,   2), S( -33,  23), S( 33,   6), S( 22,  -9),
        S( 95, -24), S( 47,  13), S( 38,   9), S( 86,   3), S(  41,  11), S(  26,  36), S(-23,  31), S(-76,  29),
        S( 49,   3), S( 65,  13), S( 76,  14), S( 29,  12), S(  43,  13), S(  89,  34), S( 92,  34), S( -6,  16),
        S(  3,  -9), S( -4,  24), S( 18,  24), S(-29,  34), S( -24,  31), S( -28,  41), S( -1,  30), S(-67,  16),
        S(-63, -10), S( 23,  -5), S(-47,  31), S(-95,  43), S( -96,  46), S( -63,  36), S(-55,  22), S(-73,   2),
        S( 15, -22), S( -2,  -1), S(-32,  19), S(-67,  33), S( -66,  37), S( -55,  30), S( -9,  12), S(-28,  -2),
        S( 21, -31), S( 23, -14), S(-10,  10), S(-72,  24), S( -53,  25), S( -21,  13), S( 23,  -6), S( 31, -23),
        S( -3, -57), S( 55, -46), S( 23, -25), S(-65,  -2), S(  11, -30), S( -32,  -6), S( 43, -33), S( 40, -58),
    ],
];
