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

    pub const PASSER: [S; 8] = [S(0, 0), S(-6, 41), S(-11, 32), S(-12, 13), S(-11, 13), S(-16, 14), S(-19, 10), S(0, 0)];

    const RAW_PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
        [
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
            S(176, 211), S(195, 203), S(179, 208), S(203, 167), S(185, 163), S(175, 170), S(101, 209), S( 76, 219),
            S( 82, 173), S( 97, 186), S(130, 150), S(140, 131), S(143, 127), S(164, 108), S(143, 151), S( 96, 150),
            S( 63, 130), S( 89, 119), S( 90, 100), S( 96,  84), S(118,  74), S(106,  85), S(112, 103), S( 87, 103),
            S( 51, 104), S( 79, 106), S( 78,  86), S( 94,  80), S( 93,  79), S( 85,  84), S( 96,  93), S( 73,  87),
            S( 53, 105), S( 81, 102), S( 79,  85), S( 75, 118), S( 90, 106), S( 83,  89), S(115,  93), S( 85,  84),
            S( 57, 108), S( 84, 105), S( 78,  93), S( 61, 124), S( 83, 128), S(102, 101), S(129,  93), S( 79,  92),
            S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        ], [
            S(163, 203), S(219, 270), S(256, 291), S(283, 285), S(321, 286), S(259, 264), S(248, 269), S(201, 202),
            S(293, 270), S(310, 289), S(342, 291), S(357, 292), S(344, 282), S(404, 270), S(310, 281), S(330, 253),
            S(307, 284), S(347, 294), S(363, 311), S(377, 308), S(413, 295), S(414, 291), S(370, 284), S(335, 270),
            S(304, 294), S(320, 310), S(344, 326), S(369, 317), S(349, 319), S(374, 317), S(330, 304), S(340, 284),
            S(289, 303), S(307, 307), S(322, 331), S(321, 326), S(332, 334), S(328, 324), S(328, 299), S(300, 295),
            S(269, 287), S(292, 306), S(310, 284), S(313, 319), S(324, 315), S(312, 300), S(317, 288), S(286, 287),
            S(256, 275), S(268, 293), S(287, 297), S(297, 318), S(300, 297), S(302, 295), S(287, 285), S(289, 261),
            S(216, 257), S(265, 260), S(253, 280), S(268, 281), S(274, 292), S(286, 275), S(262, 287), S(244, 251),
        ], [
            S(300, 310), S(289, 316), S(302, 311), S(262, 327), S(272, 319), S(286, 310), S(320, 299), S(272, 307),
            S(330, 290), S(354, 307), S(351, 313), S(332, 313), S(364, 304), S(364, 299), S(351, 310), S(340, 282),
            S(342, 315), S(367, 311), S(366, 320), S(393, 307), S(381, 311), S(407, 314), S(386, 305), S(370, 303),
            S(331, 316), S(346, 329), S(370, 320), S(381, 330), S(377, 326), S(374, 321), S(346, 312), S(329, 319),
            S(326, 304), S(340, 326), S(347, 321), S(366, 325), S(363, 325), S(347, 318), S(336, 328), S(332, 294),
            S(334, 320), S(346, 300), S(342, 329), S(348, 301), S(348, 310), S(340, 332), S(345, 304), S(347, 303),
            S(341, 279), S(334, 340), S(355, 274), S(326, 331), S(331, 351), S(351, 295), S(352, 336), S(345, 269),
            S(312, 293), S(340, 280), S(314, 308), S(309, 308), S(315, 305), S(308, 323), S(339, 285), S(325, 288),
        ], [
            S(484, 545), S(477, 552), S(483, 561), S(494, 555), S(508, 547), S(526, 535), S(509, 537), S(533, 530),
            S(468, 545), S(468, 555), S(491, 557), S(517, 545), S(503, 546), S(531, 533), S(511, 533), S(543, 519),
            S(447, 544), S(468, 547), S(471, 547), S(485, 540), S(512, 528), S(508, 525), S(546, 517), S(519, 514),
            S(430, 548), S(443, 545), S(445, 553), S(460, 547), S(465, 533), S(464, 528), S(472, 526), S(473, 522),
            S(410, 541), S(415, 546), S(421, 549), S(440, 545), S(437, 542), S(421, 542), S(447, 527), S(436, 523),
            S(403, 538), S(415, 534), S(421, 535), S(425, 539), S(430, 534), S(428, 525), S(464, 501), S(441, 504),
            S(401, 531), S(413, 534), S(429, 535), S(429, 535), S(431, 529), S(432, 526), S(449, 516), S(420, 520),
            S(419, 530), S(422, 536), S(432, 543), S(441, 532), S(445, 525), S(429, 537), S(445, 527), S(419, 526),
        ], [
            S(904, 985), S(910, 997), S(942,1012), S(974,1000), S(980, 996), S(977, 992), S(986, 959), S(949, 975),
            S(943, 955), S(926, 987), S(932,1022), S(935,1026), S(950,1032), S(980,1002), S(955, 989), S(1000, 962),
            S(947, 961), S(947, 967), S(948,1012), S(971, 997), S(983,1010), S(1013,1003), S(1011, 971), S(1013, 942),
            S(928, 969), S(934, 995), S(938,1008), S(942,1023), S(950,1025), S(961,1016), S(963, 987), S(966, 970),
            S(915,1050), S(933, 990), S(927,1013), S(943,1007), S(942,1010), S(946, 982), S(956, 968), S(956, 960),
            S(925, 970), S(924,1035), S(932, 994), S(944, 936), S(951, 935), S(954, 937), S(971, 903), S(965, 891),
            S(921, 972), S(922, 993), S(932,1021), S(952, 903), S(944, 941), S(961, 897), S(957, 904), S(971, 867),
            S(922, 967), S(908, 984), S(915, 983), S(922,1055), S(928, 950), S(917, 948), S(938, 915), S(930, 921),
        ], [
            S(-21, -64), S(-23, -24), S( -1, -14), S(-45,  12), S(-23,  -0), S( -4,   9), S( 13,  14), S(  8, -63),
            S(-49,   2), S(-37,  33), S(-64,  43), S(  3,  32), S(-16,  46), S(-12,  56), S( 14,  50), S( -1,  21),
            S(-75,  17), S( -4,  40), S(-54,  56), S(-61,  63), S(-29,  63), S( 22,  60), S( 11,  59), S(-20,  31),
            S(-70,  13), S(-75,  44), S(-86,  61), S(-114, 69), S(-108, 70), S(-83,  66), S(-85,  60), S(-104, 36),
            S(-69,   1), S(-80,  33), S(-109, 56), S(-132, 71), S(-134, 70), S(-103, 59), S(-105, 47), S(-124, 29),
            S(-33,  -6), S(-18,  15), S(-77,  35), S(-87,  49), S(-81,  50), S(-80,  39), S(-33,  21), S(-51,   9),
            S( 58, -30), S( 13,  -1), S( -2,  10), S(-37,  20), S(-40,  23), S(-22,  16), S( 27,  -3), S( 38, -21),
            S( 53, -65), S( 78, -51), S( 45, -23), S(-60,  -5), S(  6, -16), S(-33,  -8), S( 52, -35), S( 55, -68),
        ],
    ];
}
