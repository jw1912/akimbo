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

    pub const PASSER: [S; 7] = [S(0, 0), S(-10, -3), S(-17, -1), S(-15, 25), S(9, 46), S(16, 87), S(29, 73)];
    pub const OPEN: [S; 8] = [S(35, 26), S(29, 21), S(21, 23), S(20, 11), S(18, 3), S(5, 7), S(7, 5), S(23, 2)];
    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(134, 188), S(149, 174), S(128, 178), S(163, 127), S(137, 127), S(129, 137), S( 54, 178), S( 35, 194),
            S( 79, 141), S( 91, 148), S(117, 105), S(120,  75), S(128,  74), S(165,  81), S(138, 120), S( 97, 124),
            S( 60, 122), S( 84, 115), S( 84,  94), S( 89,  71), S(112,  68), S(102,  82), S(107, 103), S( 85,  98),
            S( 49, 106), S( 75, 111), S( 72,  90), S( 89,  82), S( 86,  82), S( 79,  91), S( 92, 100), S( 69,  91),
            S( 46, 110), S( 71, 108), S( 67,  90), S( 63, 117), S( 77, 111), S( 72,  95), S(105, 100), S( 76,  89),
            S( 46, 108), S( 70, 106), S( 63,  93), S( 46, 115), S( 67, 125), S( 88, 102), S(116,  95), S( 66,  94),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(162, 210), S(216, 272), S(253, 296), S(282, 289), S(318, 290), S(256, 269), S(246, 270), S(201, 206),
            S(290, 277), S(308, 293), S(340, 295), S(356, 296), S(343, 286), S(401, 274), S(309, 285), S(325, 261),
            S(308, 287), S(346, 297), S(362, 315), S(377, 312), S(413, 298), S(416, 292), S(369, 289), S(338, 273),
            S(305, 299), S(321, 315), S(345, 330), S(370, 321), S(350, 322), S(376, 320), S(332, 308), S(342, 287),
            S(290, 308), S(307, 313), S(324, 335), S(323, 331), S(333, 340), S(330, 327), S(331, 303), S(302, 300),
            S(270, 292), S(294, 311), S(312, 289), S(315, 325), S(327, 320), S(314, 305), S(319, 293), S(288, 292),
            S(258, 281), S(269, 299), S(289, 303), S(299, 322), S(302, 301), S(305, 300), S(290, 290), S(292, 264),
            S(218, 266), S(266, 265), S(255, 286), S(271, 286), S(276, 296), S(290, 280), S(264, 292), S(247, 261),
        ], [
            S(303, 314), S(289, 323), S(300, 315), S(260, 331), S(271, 323), S(285, 314), S(319, 307), S(275, 312),
            S(331, 297), S(353, 312), S(350, 318), S(332, 317), S(362, 309), S(361, 305), S(350, 314), S(340, 288),
            S(342, 319), S(366, 316), S(366, 323), S(390, 311), S(380, 315), S(406, 319), S(385, 311), S(370, 309),
            S(331, 322), S(347, 333), S(369, 324), S(383, 334), S(378, 332), S(375, 326), S(348, 317), S(330, 324),
            S(327, 311), S(340, 330), S(348, 326), S(367, 331), S(364, 331), S(349, 322), S(338, 334), S(334, 299),
            S(333, 325), S(348, 305), S(344, 335), S(350, 307), S(350, 316), S(342, 336), S(347, 308), S(348, 308),
            S(342, 284), S(336, 344), S(357, 279), S(328, 334), S(333, 355), S(353, 300), S(355, 339), S(347, 273),
            S(315, 297), S(343, 284), S(316, 315), S(313, 313), S(317, 309), S(310, 327), S(340, 290), S(329, 291),
        ], [
            S(458, 556), S(451, 561), S(455, 569), S(464, 563), S(480, 556), S(500, 547), S(486, 550), S(508, 541),
            S(456, 554), S(455, 563), S(476, 564), S(500, 552), S(487, 552), S(513, 542), S(497, 542), S(528, 530),
            S(438, 550), S(461, 550), S(461, 551), S(474, 543), S(501, 531), S(501, 529), S(540, 522), S(512, 520),
            S(419, 555), S(434, 551), S(432, 557), S(445, 551), S(451, 537), S(457, 532), S(466, 533), S(464, 528),
            S(405, 544), S(410, 547), S(411, 549), S(428, 542), S(426, 540), S(417, 543), S(444, 530), S(432, 527),
            S(399, 535), S(409, 530), S(412, 528), S(417, 529), S(423, 525), S(426, 522), S(463, 499), S(439, 502),
            S(398, 528), S(406, 531), S(415, 531), S(417, 525), S(421, 522), S(430, 522), S(446, 513), S(417, 517),
            S(415, 535), S(417, 532), S(419, 535), S(430, 515), S(436, 511), S(427, 537), S(441, 524), S(417, 528),
        ], [
            S(910, 998), S(917,1008), S(946,1024), S(978,1014), S(982,1011), S(984,1002), S(996, 969), S(955, 987),
            S(951, 968), S(931, 999), S(935,1035), S(937,1040), S(952,1045), S(983,1013), S(958,1005), S(1004, 977),
            S(954, 973), S(951, 980), S(953,1023), S(973,1010), S(988,1022), S(1021,1011), S(1020, 981), S(1022, 952),
            S(935, 983), S(942,1007), S(944,1020), S(949,1034), S(956,1037), S(969,1027), S(971,1000), S(974, 982),
            S(923,1061), S(941,1001), S(934,1023), S(950,1018), S(949,1020), S(955, 992), S(965, 980), S(964, 973),
            S(933, 980), S(932,1045), S(940,1004), S(952, 946), S(959, 947), S(963, 947), S(979, 915), S(974, 902),
            S(930, 982), S(931,1002), S(940,1030), S(961, 912), S(952, 950), S(971, 906), S(967, 911), S(979, 876),
            S(930, 978), S(918, 995), S(924, 994), S(929,1067), S(937, 962), S(927, 958), S(947, 924), S(939, 933),
        ], [
            S(-22, -66), S(-23, -25), S( -5, -13), S(-46,  13), S(-27,   1), S( -4,   8), S( 19,  13), S( 12, -64),
            S(-52,   0), S(-40,  33), S(-65,  42), S( -1,  33), S(-19,  45), S(-18,  55), S( 16,  47), S(  3,  19),
            S(-76,  14), S( -2,  37), S(-53,  54), S(-66,  62), S(-32,  62), S( 21,  57), S( 11,  55), S(-18,  26),
            S(-70,  11), S(-73,  41), S(-89,  60), S(-121,  69), S(-115,  70), S(-86,  66), S(-84,  58), S(-102,  32),
            S(-73,   1), S(-78,  32), S(-112,  56), S(-141,  73), S(-138,  71), S(-104,  59), S(-103,  45), S(-128,  30),
            S(-30,  -7), S(-15,  16), S(-77,  37), S(-90,  51), S(-81,  52), S(-80,  41), S(-33,  22), S(-51,  11),
            S( 62, -27), S( 16,   1), S(  0,  11), S(-37,  21), S(-40,  25), S(-22,  18), S( 27,  -1), S( 38, -19),
            S( 55, -65), S( 79, -51), S( 47, -23), S(-58,  -4), S(  5, -15), S(-31,  -7), S( 52, -35), S( 56, -68),
        ],
    ];
}
