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

    pub const PASSER: [S; 7] = [S(0, 0), S(-11, -2), S(-16, 3), S(-14, 24), S(10, 45), S(13, 94), S(26, 77)];
    pub const OPEN: [S; 8] = [S(37, 4), S(28, 4), S(25, 11), S(22, 18), S(31, 19), S(48, 4), S(55, 1), S(116, -13)];
    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(134, 188), S(149, 178), S(130, 177), S(157, 131), S(135, 131), S(122, 141), S( 47, 183), S( 35, 195),
            S( 79, 141), S( 92, 143), S(118, 102), S(118,  73), S(126,  69), S(160,  82), S(133, 120), S( 99, 124),
            S( 59, 124), S( 82, 116), S( 83,  95), S( 85,  80), S(107,  79), S(100,  85), S(105, 105), S( 84, 102),
            S( 48, 110), S( 74, 110), S( 71,  92), S( 89,  87), S( 88,  87), S( 80,  91), S( 92, 101), S( 70,  93),
            S( 46, 106), S( 69, 108), S( 67,  91), S( 68,  96), S( 82,  96), S( 73,  94), S(106,  98), S( 75,  90),
            S( 46, 110), S( 69, 112), S( 63,  98), S( 52,  95), S( 72, 106), S( 89,  98), S(115,  97), S( 67,  91),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(155, 229), S(212, 278), S(252, 301), S(284, 292), S(313, 295), S(254, 276), S(240, 279), S(198, 217),
            S(289, 282), S(309, 297), S(336, 304), S(353, 303), S(337, 296), S(398, 282), S(308, 294), S(322, 267),
            S(308, 292), S(343, 305), S(360, 321), S(374, 321), S(411, 306), S(416, 299), S(366, 297), S(337, 281),
            S(305, 302), S(319, 323), S(345, 333), S(367, 334), S(348, 336), S(374, 328), S(330, 321), S(340, 294),
            S(291, 306), S(307, 315), S(323, 335), S(324, 337), S(333, 339), S(329, 327), S(329, 315), S(302, 296),
            S(272, 290), S(296, 308), S(309, 318), S(314, 330), S(326, 328), S(314, 313), S(318, 302), S(289, 292),
            S(258, 284), S(271, 298), S(288, 305), S(300, 307), S(302, 307), S(305, 303), S(292, 287), S(289, 291),
            S(218, 274), S(268, 262), S(257, 291), S(272, 293), S(277, 292), S(292, 283), S(271, 271), S(246, 269),
        ], [
            S(306, 316), S(291, 326), S(300, 322), S(260, 335), S(271, 328), S(285, 320), S(317, 315), S(279, 314),
            S(330, 303), S(354, 318), S(347, 322), S(331, 324), S(358, 315), S(356, 315), S(349, 321), S(337, 300),
            S(341, 326), S(364, 321), S(365, 329), S(387, 319), S(376, 323), S(407, 325), S(383, 319), S(370, 320),
            S(332, 324), S(348, 337), S(369, 331), S(382, 342), S(377, 337), S(373, 334), S(349, 335), S(333, 323),
            S(326, 321), S(339, 335), S(346, 342), S(367, 339), S(364, 338), S(349, 337), S(340, 332), S(335, 309),
            S(336, 318), S(345, 328), S(345, 335), S(348, 336), S(349, 340), S(344, 334), S(347, 318), S(350, 308),
            S(340, 313), S(341, 313), S(353, 311), S(330, 325), S(338, 327), S(352, 315), S(359, 316), S(344, 293),
            S(318, 296), S(339, 313), S(321, 296), S(314, 315), S(318, 311), S(317, 313), S(343, 300), S(331, 281),
        ], [
            S(442, 570), S(442, 575), S(451, 576), S(454, 566), S(464, 560), S(474, 565), S(467, 566), S(434, 572),
            S(425, 572), S(432, 580), S(453, 576), S(475, 558), S(451, 561), S(469, 563), S(457, 562), S(437, 563),
            S(409, 570), S(442, 568), S(440, 565), S(443, 555), S(465, 544), S(477, 547), S(513, 541), S(457, 545),
            S(405, 570), S(425, 566), S(424, 569), S(432, 558), S(433, 546), S(442, 549), S(451, 548), S(432, 546),
            S(394, 562), S(398, 566), S(404, 563), S(417, 554), S(414, 551), S(405, 558), S(431, 548), S(412, 544),
            S(389, 557), S(398, 557), S(403, 552), S(406, 549), S(411, 544), S(411, 546), S(448, 526), S(420, 528),
            S(389, 551), S(398, 556), S(409, 553), S(409, 547), S(412, 539), S(419, 542), S(435, 534), S(396, 540),
            S(408, 551), S(409, 556), S(413, 560), S(422, 550), S(425, 543), S(419, 549), S(431, 545), S(407, 539),
        ], [
            S(907,1017), S(923,1022), S(957,1032), S(986,1023), S(982,1026), S(995,1008), S(1004, 976), S(950,1007),
            S(951, 980), S(927,1020), S(937,1046), S(935,1057), S(944,1066), S(972,1037), S(951,1027), S(989,1008),
            S(953, 986), S(949,1003), S(949,1040), S(964,1041), S(976,1046), S(1017,1028), S(1016, 996), S(1013, 986),
            S(935, 998), S(941,1018), S(945,1030), S(945,1051), S(950,1059), S(963,1045), S(961,1035), S(966,1015),
            S(939, 991), S(937,1019), S(936,1027), S(945,1044), S(944,1043), S(944,1034), S(956,1016), S(957,1005),
            S(936, 977), S(944, 994), S(939,1017), S(938,1016), S(941,1019), S(949,1010), S(962, 987), S(955, 974),
            S(935, 973), S(941, 975), S(951, 972), S(950, 982), S(948, 986), S(959, 958), S(965, 930), S(970, 904),
            S(936, 963), S(924, 973), S(930, 976), S(945, 970), S(937, 973), S(926, 967), S(947, 939), S(940, 938),
        ], [
            S(-26, -80), S(-19, -40), S( -1, -27), S(-45,   2), S(-23,  -9), S(  6,  -4), S( 27,  -4), S( 12, -79),
            S(-57, -12), S(-36,  21), S(-61,  30), S(  3,  20), S(-17,  33), S(-13,  43), S( 21,  34), S(  7,   6),
            S(-81,   4), S(  1,  25), S(-54,  42), S(-68,  51), S(-34,  51), S( 26,  45), S( 12,  43), S(-22,  15),
            S(-68,  -2), S(-77,  30), S(-87,  48), S(-122,  59), S(-116,  59), S(-84,  54), S(-82,  45), S(-105,  20),
            S(-76,  -9), S(-78,  19), S(-111,  43), S(-140,  58), S(-134,  57), S(-100,  44), S(-102,  32), S(-129,  18),
            S(-33, -17), S(-17,   5), S(-73,  26), S(-88,  38), S(-80,  37), S(-78,  29), S(-35,  11), S(-53,   1),
            S( 56, -35), S( 13,  -9), S( -1,   2), S(-37,  12), S(-39,  15), S(-21,   7), S( 27, -10), S( 35, -28),
            S( 46, -69), S( 72, -52), S( 45, -33), S(-57, -14), S(  8, -38), S(-31, -16), S( 50, -43), S( 52, -70),
        ],
    ];
}
