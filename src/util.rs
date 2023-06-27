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

impl std::ops::Mul<S> for i32 {
    type Output = S;
    fn mul(self, rhs: S) -> Self::Output {
        S(self * rhs.0, self * rhs.1)
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
pub static PST: [[[S; 64]; 8]; 2] = [
        init!(i, 8, init!(j, 64, Eval::RAW_PST[i][j ^ 56])),
        init!(i, 8, init!(j, 64, S(-Eval::RAW_PST[i][j].0, -Eval::RAW_PST[i][j].1))),
];

pub struct Eval;
impl Eval {
    pub const SIDE: [i32; 2] = [1, -1];
    pub const PHASE: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];

    pub const PASSER: [S; 7] = [S(0, 0), S(-9, -4), S(-17, -2), S(-15, 24), S(9, 46), S(16, 86), S(30, 72)];
    pub const OPEN: [S; 8] = [S(0, 0); 8];
    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(134, 187), S(150, 173), S(129, 178), S(163, 128), S(139, 128), S(128, 136), S( 56, 177), S( 33, 193),
            S( 77, 141), S( 90, 148), S(114, 105), S(117,  75), S(125,  74), S(162,  81), S(139, 119), S( 96, 124),
            S( 59, 122), S( 83, 115), S( 83,  94), S( 88,  71), S(111,  68), S(100,  82), S(107, 103), S( 84,  98),
            S( 48, 106), S( 74, 111), S( 72,  90), S( 89,  82), S( 87,  82), S( 79,  91), S( 91,  99), S( 69,  91),
            S( 45, 109), S( 70, 107), S( 68,  90), S( 64, 118), S( 78, 111), S( 72,  95), S(105,  99), S( 75,  89),
            S( 45, 108), S( 70, 106), S( 64,  93), S( 46, 117), S( 68, 126), S( 87, 102), S(116,  95), S( 66,  94),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(161, 207), S(219, 271), S(256, 294), S(283, 288), S(321, 288), S(258, 268), S(247, 269), S(201, 204),
            S(292, 276), S(309, 292), S(342, 294), S(358, 295), S(345, 285), S(405, 273), S(309, 285), S(328, 259),
            S(308, 286), S(347, 296), S(363, 313), S(377, 311), S(414, 297), S(416, 291), S(370, 288), S(336, 272),
            S(305, 298), S(321, 314), S(345, 329), S(370, 320), S(349, 321), S(375, 319), S(331, 307), S(340, 287),
            S(290, 307), S(307, 312), S(323, 334), S(322, 329), S(332, 338), S(328, 326), S(329, 302), S(301, 298),
            S(269, 290), S(292, 310), S(311, 288), S(314, 323), S(325, 318), S(313, 304), S(317, 290), S(286, 289),
            S(257, 280), S(268, 297), S(287, 301), S(297, 320), S(300, 300), S(303, 298), S(287, 288), S(290, 263),
            S(216, 263), S(266, 263), S(253, 284), S(269, 284), S(274, 294), S(287, 277), S(263, 289), S(244, 259),
        ], [
            S(301, 312), S(289, 321), S(301, 314), S(262, 330), S(274, 321), S(286, 313), S(321, 305), S(273, 311),
            S(329, 296), S(352, 310), S(350, 317), S(332, 316), S(364, 307), S(363, 303), S(350, 313), S(339, 286),
            S(342, 317), S(367, 314), S(366, 322), S(392, 309), S(380, 314), S(407, 318), S(385, 309), S(370, 307),
            S(331, 320), S(347, 331), S(370, 322), S(382, 332), S(378, 330), S(375, 324), S(347, 315), S(330, 322),
            S(326, 310), S(340, 329), S(347, 324), S(366, 330), S(363, 329), S(348, 321), S(337, 332), S(333, 298),
            S(334, 324), S(347, 303), S(342, 333), S(349, 305), S(348, 314), S(341, 334), S(346, 307), S(347, 306),
            S(341, 283), S(334, 343), S(355, 277), S(326, 333), S(332, 353), S(351, 298), S(353, 338), S(346, 272),
            S(313, 296), S(340, 283), S(315, 312), S(310, 311), S(315, 307), S(308, 325), S(339, 288), S(326, 290),
        ], [
            S(482, 551), S(475, 558), S(481, 566), S(492, 561), S(507, 553), S(524, 542), S(505, 546), S(532, 536),
            S(467, 552), S(466, 562), S(490, 563), S(515, 552), S(502, 552), S(529, 539), S(509, 539), S(541, 526),
            S(445, 551), S(467, 551), S(469, 552), S(484, 545), S(511, 533), S(508, 529), S(547, 521), S(518, 520),
            S(428, 555), S(442, 551), S(444, 557), S(459, 552), S(464, 538), S(463, 533), S(472, 533), S(471, 527),
            S(408, 549), S(415, 552), S(419, 555), S(438, 551), S(435, 548), S(420, 548), S(445, 534), S(434, 532),
            S(402, 546), S(413, 541), S(420, 541), S(424, 546), S(428, 540), S(426, 531), S(463, 507), S(439, 511),
            S(400, 537), S(411, 540), S(427, 540), S(428, 539), S(429, 534), S(431, 531), S(448, 522), S(419, 527),
            S(418, 536), S(421, 541), S(431, 548), S(440, 537), S(443, 530), S(428, 543), S(444, 533), S(418, 532),
        ], [
            S(907, 994), S(914,1004), S(945,1018), S(977,1008), S(981,1006), S(982, 997), S(991, 964), S(953, 982),
            S(947, 964), S(928, 995), S(935,1028), S(937,1033), S(952,1039), S(983,1008), S(955,1000), S(1003, 971),
            S(951, 967), S(951, 974), S(951,1017), S(974,1002), S(986,1016), S(1017,1008), S(1016, 977), S(1018, 948),
            S(932, 978), S(938,1003), S(941,1015), S(946,1030), S(953,1032), S(965,1023), S(966, 996), S(969, 978),
            S(919,1058), S(936, 998), S(930,1019), S(946,1014), S(945,1017), S(950, 990), S(960, 975), S(959, 969),
            S(929, 976), S(928,1041), S(936,1000), S(948, 943), S(955, 943), S(958, 943), S(975, 909), S(969, 898),
            S(925, 979), S(926, 999), S(936,1027), S(956, 908), S(948, 947), S(965, 903), S(962, 907), S(975, 872),
            S(925, 974), S(912, 989), S(919, 988), S(925,1059), S(932, 956), S(921, 952), S(941, 919), S(934, 928),
        ], [
            S(-23, -66), S(-22, -25), S( -5, -12), S(-45,  13), S(-26,   0), S( -5,   8), S( 18,  13), S( 11, -63),
            S(-52,  -0), S(-41,  32), S(-64,  42), S( -3,  32), S(-20,  44), S(-17,  55), S( 17,  46), S(  2,  19),
            S(-76,  14), S( -3,  37), S(-53,  54), S(-64,  62), S(-32,  61), S( 23,  57), S( 12,  55), S(-18,  26),
            S(-71,  11), S(-73,  41), S(-87,  59), S(-119, 68), S(-113, 69), S(-86,  65), S(-84,  57), S(-103, 32),
            S(-76,   1), S(-81,  32), S(-113,  56), S(-139,73), S(-137, 71), S(-105, 59), S(-105, 45), S(-128, 29),
            S(-33,  -6), S(-19,  16), S(-79,  37), S(-89,  51), S(-81,  52), S(-80,  41), S(-34,  22), S(-52,  10),
            S( 60, -28), S( 13,   1), S( -2,  11), S(-37,  22), S(-40,  24), S(-22,  18), S( 27,  -1), S( 39, -20),
            S( 54, -65), S( 77, -50), S( 45, -21), S(-60,  -4), S(  6, -15), S(-33,  -7), S( 53, -34), S( 56, -67),
        ],
    ];
}
