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
pub static PST: [[[S; 64]; 8]; 2] = [
        init!(i, 8, init!(j, 64, Eval::RAW_PST[i][j ^ 56])),
        init!(i, 8, init!(j, 64, S(-Eval::RAW_PST[i][j].0, -Eval::RAW_PST[i][j].1))),
];

pub struct Eval;
impl Eval {
    pub const SIDE: [i32; 2] = [1, -1];
    pub const PHASE: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];

    pub const PASSER: [S; 8] = [S(0, 0); 8];

    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(190, 238), S(198, 230), S(180, 233), S(198, 195), S(179, 192), S(169, 198), S( 97, 236), S( 77, 248),
            S( 72, 203), S( 86, 213), S(118, 178), S(127, 157), S(130, 153), S(154, 135), S(133, 178), S( 86, 179),
            S( 52, 143), S( 78, 132), S( 79, 113), S( 85,  98), S(107,  87), S( 96,  98), S(101, 116), S( 76, 116),
            S( 40, 117), S( 68, 119), S( 67,  99), S( 83,  93), S( 82,  92), S( 74,  97), S( 85, 106), S( 62, 100),
            S( 37, 119), S( 65, 115), S( 63,  98), S( 59, 131), S( 74, 119), S( 68, 102), S( 99, 106), S( 69,  97),
            S( 38, 118), S( 65, 115), S( 59, 103), S( 42, 133), S( 64, 138), S( 83, 110), S(110, 103), S( 60, 102),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(163, 203), S(218, 269), S(257, 289), S(284, 284), S(322, 284), S(260, 262), S(247, 268), S(202, 201),
            S(294, 269), S(311, 288), S(343, 290), S(358, 291), S(346, 281), S(405, 268), S(311, 280), S(331, 252),
            S(308, 283), S(348, 293), S(364, 310), S(378, 307), S(415, 293), S(415, 289), S(371, 283), S(336, 269),
            S(305, 293), S(321, 309), S(345, 324), S(370, 316), S(350, 317), S(375, 315), S(332, 302), S(341, 283),
            S(290, 301), S(308, 306), S(323, 329), S(323, 324), S(333, 333), S(329, 322), S(329, 298), S(301, 294),
            S(270, 286), S(293, 305), S(311, 283), S(314, 318), S(326, 314), S(313, 299), S(318, 286), S(287, 285),
            S(258, 274), S(269, 292), S(288, 296), S(298, 317), S(301, 296), S(304, 293), S(288, 284), S(290, 260),
            S(218, 256), S(266, 259), S(254, 279), S(269, 280), S(275, 291), S(287, 274), S(263, 286), S(246, 249),
        ], [
            S(302, 308), S(290, 315), S(303, 309), S(263, 325), S(273, 317), S(288, 308), S(321, 298), S(274, 305),
            S(331, 289), S(354, 306), S(352, 312), S(333, 312), S(365, 303), S(365, 297), S(352, 309), S(342, 280),
            S(343, 314), S(368, 310), S(367, 319), S(394, 306), S(382, 310), S(408, 313), S(387, 304), S(371, 302),
            S(332, 315), S(347, 328), S(371, 319), S(382, 328), S(378, 325), S(375, 320), S(347, 311), S(331, 317),
            S(327, 303), S(341, 325), S(348, 320), S(367, 324), S(364, 324), S(348, 317), S(337, 327), S(333, 292),
            S(335, 319), S(347, 298), S(343, 328), S(349, 300), S(349, 309), S(341, 330), S(347, 303), S(348, 302),
            S(342, 278), S(335, 338), S(356, 273), S(327, 330), S(332, 350), S(352, 294), S(353, 335), S(347, 268),
            S(314, 292), S(341, 279), S(315, 307), S(311, 307), S(316, 303), S(309, 322), S(340, 283), S(327, 287),
        ], [
            S(485, 544), S(478, 551), S(484, 559), S(495, 554), S(509, 545), S(528, 533), S(510, 536), S(534, 529),
            S(469, 544), S(469, 554), S(492, 556), S(518, 544), S(504, 545), S(531, 532), S(511, 532), S(544, 518),
            S(448, 543), S(469, 546), S(471, 546), S(486, 539), S(512, 527), S(508, 524), S(547, 516), S(520, 514),
            S(430, 547), S(444, 544), S(446, 552), S(461, 546), S(466, 532), S(464, 527), S(473, 525), S(473, 521),
            S(411, 540), S(416, 545), S(422, 548), S(441, 544), S(438, 541), S(422, 541), S(448, 526), S(437, 522),
            S(404, 538), S(415, 533), S(422, 534), S(426, 538), S(430, 533), S(428, 524), S(464, 500), S(441, 503),
            S(402, 530), S(414, 533), S(429, 534), S(430, 533), S(431, 528), S(433, 525), S(450, 515), S(421, 519),
            S(420, 529), S(423, 535), S(433, 542), S(442, 530), S(445, 524), S(430, 536), S(446, 526), S(419, 525),
        ], [
            S(905, 984), S(910, 997), S(943,1011), S(975,1000), S(981, 995), S( 977, 991), S( 987, 957), S( 949, 974),
            S(943, 955), S(926, 987), S(932,1022), S(935,1025), S(949,1032), S( 980,1001), S( 955, 989), S(1000, 960),
            S(947, 961), S(947, 967), S(948,1011), S(971, 996), S(983,1010), S(1013,1002), S(1012, 970), S(1014, 940),
            S(928, 969), S(934, 995), S(938,1008), S(942,1023), S(950,1025), S( 961,1016), S( 963, 987), S( 966, 970),
            S(915,1050), S(933, 990), S(926,1013), S(943,1007), S(941,1009), S( 946, 982), S( 956, 967), S( 956, 960),
            S(925, 970), S(924,1035), S(932, 993), S(944, 935), S(951, 935), S( 954, 936), S( 971, 903), S( 965, 891),
            S(921, 972), S(922, 992), S(931,1021), S(952, 903), S(944, 941), S( 961, 897), S( 957, 903), S( 971, 866),
            S(922, 967), S(908, 984), S(915, 984), S(921,1054), S(928, 950), S( 917, 948), S( 938, 914), S( 930, 919),
        ], [
            S(-20, -62), S(-24, -22), S(  -2, -13), S( -46,  13), S( -22,  -0), S(  -4,   9), S(  12,  15), S(   8, -62),
            S(-52,   2), S(-38,  34), S( -65,  44), S(   3,  32), S( -17,  46), S( -13,  56), S(  12,  51), S(  -5,  22),
            S(-76,  17), S( -4,  40), S( -54,  56), S( -62,  64), S( -31,  64), S(  20,  60), S(   9,  59), S( -23,  31),
            S(-72,  14), S(-75,  44), S( -86,  61), S(-115,  69), S(-110,  70), S( -85,  67), S( -86,  60), S(-106,  36),
            S(-70,   1), S(-80,  33), S(-109,  56), S(-133,  71), S(-134,  70), S(-104,  59), S(-105,  46), S(-125,  29),
            S(-34,  -6), S(-18,  15), S( -77,  35), S( -87,  49), S( -81,  50), S( -80,  39), S( -33,  21), S( -51,   9),
            S( 58, -30), S( 13,  -1), S(  -2,   9), S( -37,  20), S( -40,  23), S( -22,  16), S(  27,  -3), S(  38, -22),
            S( 53, -65), S( 78, -52), S(  45, -23), S( -60,  -5), S(   6, -16), S( -33,  -8), S(  52, -35), S(  56, -68),
        ],
    ];
}
