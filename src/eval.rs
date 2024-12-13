use std::ops::{AddAssign, Sub};

use crate::{attacks::Attacks, bitloop, consts::{File, Piece, Side, MOBILITY_OFFSET, RAILS, SPANS}, position::Position};

pub fn eval(pos: &Position) -> i32 {
    let white = SideEvalState::new(pos, Side::WHITE);
    let black = SideEvalState::new(pos, Side::BLACK);

    let score = white.eval() - black.eval();

    [1, -1][pos.stm()] * score.taper(pos.phase)
}

pub struct SideEvalState<'a> {
    side: usize,
    pos: &'a Position,
    flip: usize,
    safe: u64,
    occ: u64,
}

impl<'a> SideEvalState<'a> {
    fn new(pos: &'a Position, side: usize) -> Self {
        let ksq = (pos.piece(Piece::KING) & pos.side(side)).trailing_zeros();
        let flip = [56, 0][side] ^ if ksq % 8 > 3 { 7 } else { 0 };

        let pawn_threats = if side == Side::WHITE {
            Attacks::black_pawn_setwise(pos.side(side ^ 1) & pos.piece(Piece::PAWN))
        } else {
            Attacks::white_pawn_setwise(pos.side(side ^ 1) & pos.piece(Piece::PAWN))
        };
    
        let safe = !pawn_threats;
        let occ = pos.side(side) ^ pos.side(side ^ 1);

        Self { side, pos, flip, safe, occ }
    }

    fn piece(&self, side: usize, piece: usize) -> u64 {
        self.pos.side(side) & self.pos.piece(piece)
    }

    fn eval(&self) -> S {
        let mut score = S(0, 0);
        
        score += self.pawn_structure();
        score += self.pawn_defends();
    
        if self.piece(self.side, Piece::BISHOP).count_ones() > 1 {
            score += BISHOP_PAIR;
        }
    
        for piece in Piece::KNIGHT..=Piece::KING {
            score += self.piece_eval(piece);
        }
    
        score
    }

    fn pawn_structure(&self) -> S {
        let mut score = S(0, 0);

        let mut pawns = self.pos.side(self.side) & self.pos.piece(Piece::PAWN);
        let mut phalanx = pawns & ((pawns & !File::A) >> 1);

        bitloop!(|phalanx, sq| score += PHALANX_PAWNS[(usize::from(sq) ^ self.flip) / 8]);

        bitloop!(|pawns, sq| {
            let sq = usize::from(sq);
            let fsq = sq ^ self.flip;

            score += PST[Piece::PAWN][fsq];

            if RAILS[sq % 8] & self.piece(self.side, Piece::PAWN) == 0 {
                score += ISOLATED_PAWN_FILE[fsq % 8];
            }

            if SPANS[self.side][sq] & self.piece(self.side ^ 1, Piece::PAWN) == 0 {
                score += PASSED_PAWN_PST[fsq];
            }
        });

        score
    }

    fn pawn_defends(&self) -> S {
        let mut score = S(0, 0);

        let pawn_defends = if self.side == Side::WHITE {
            Attacks::white_pawn_setwise(self.piece(Side::WHITE, Piece::PAWN))
        } else {
            Attacks::black_pawn_setwise(self.piece(Side::BLACK, Piece::PAWN))
        };
    
        for piece in Piece::PAWN..=Piece::KING {
            let defended = (pawn_defends & self.piece(self.side, piece)).count_ones();
            score += PAWN_DEFENDS[8 * (piece - 2) + defended as usize];
        }

        score
    }

    fn piece_eval(&self, piece: usize) -> S {
        let mut score = S(0, 0);
        let mut bb = self.piece(self.side, piece);
    
        let mobility_offset = MOBILITY_OFFSET[piece];

        bitloop!(|bb, sq| {
            let sq = usize::from(sq);
            let fsq = sq ^ self.flip;

            score += PST[piece][fsq];

            if mobility_offset != usize::MAX {
                let attacks = Attacks::for_piece(piece, self.side, sq, self.occ);
                let mobility = (attacks & self.safe).count_ones() as usize;
                score += MOBILITY[mobility_offset + mobility];
            }
            
            if piece == Piece::ROOK {
                let file_bb = File::A << (sq % 8);
                    
                if file_bb & self.piece(self.side, Piece::PAWN) == 0 {
                    score += ROOK_SEMI_OPEN_FILE[fsq % 8];
                }
    
                if file_bb & self.pos.piece(Piece::PAWN) == 0 {
                    score += ROOK_FULL_OPEN_FILE[fsq % 8];
                }
            }
        });

        score
    }
}

#[derive(Clone, Copy, Default)]
pub struct S(i16, i16);

impl AddAssign<S> for S {
    fn add_assign(&mut self, rhs: S) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Sub<S> for S {
    type Output = S;

    fn sub(mut self, rhs: S) -> S {
        self.0 -= rhs.0;
        self.1 -= rhs.1;

        self
    }
}

impl S {
    fn taper(self, mut phase: i32) -> i32 {
        let mg = i32::from(self.0);
        let eg = i32::from(self.1);

        phase = phase.min(24);

        (mg * phase + (24 - phase) * eg) / 24
    }
}

static PST: [[S; 64]; 8] = [[S(0, 0); 64], [S(0, 0); 64],
[
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
        S(210, 142), S(111, 164), S(193, 140), S(194, 131), S(180, 131), S(192, 135), S(170, 148), S(194, 150),
        S(173,  83), S(161,  85), S(169,  82), S(120, 105), S( 93, 102), S(102,  96), S( 90,  96), S( 70, 120),
        S(131,  76), S( 97,  84), S(111,  74), S( 98,  82), S( 80,  87), S( 68,  89), S( 65,  88), S( 59, 103),
        S( 99,  71), S( 86,  76), S( 98,  74), S( 80,  78), S( 66,  85), S( 62,  82), S( 49,  79), S( 47,  92),
        S(108,  69), S(105,  68), S( 88,  80), S( 69,  91), S( 52,  96), S( 45,  87), S( 44,  76), S( 37,  91),
        S(124,  67), S(148,  64), S(144,  78), S( 82, 104), S( 67,  93), S( 65,  90), S( 58,  80), S( 53,  97),
        S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100), S(100, 100),
    ], [
        S(225, 275), S(303, 302), S(201, 342), S(330, 312), S(256, 326), S(205, 342), S(234, 322), S(201, 310),
        S(326, 303), S(296, 327), S(392, 289), S(373, 305), S(355, 316), S(352, 301), S(298, 323), S(292, 316),
        S(348, 301), S(365, 301), S(422, 298), S(380, 312), S(363, 313), S(356, 310), S(331, 303), S(295, 311),
        S(358, 311), S(325, 315), S(363, 321), S(351, 323), S(377, 316), S(340, 322), S(320, 308), S(319, 317),
        S(328, 311), S(338, 300), S(329, 315), S(334, 322), S(328, 321), S(322, 317), S(319, 305), S(313, 308),
        S(289, 296), S(317, 291), S(303, 292), S(321, 305), S(314, 310), S(296, 295), S(296, 294), S(288, 291),
        S(309, 315), S(314, 297), S(317, 283), S(307, 295), S(314, 290), S(298, 286), S(278, 306), S(283, 287),
        S(276, 284), S(305, 285), S(306, 287), S(297, 293), S(289, 292), S(268, 284), S(299, 275), S(268, 277),
    ], [
        S(300, 319), S(295, 324), S(185, 335), S(230, 330), S(229, 332), S(225, 327), S(229, 332), S(258, 328),
        S(320, 313), S(298, 316), S(343, 312), S(316, 315), S(306, 323), S(320, 320), S(323, 312), S(301, 320),
        S(347, 317), S(361, 314), S(368, 312), S(367, 316), S(347, 311), S(330, 320), S(343, 312), S(322, 318),
        S(326, 320), S(337, 317), S(355, 316), S(354, 316), S(373, 316), S(342, 309), S(335, 318), S(314, 319),
        S(349, 308), S(332, 312), S(325, 314), S(358, 313), S(352, 315), S(333, 321), S(324, 318), S(328, 308),
        S(345, 315), S(336, 307), S(337, 312), S(341, 316), S(338, 314), S(325, 314), S(336, 312), S(332, 309),
        S(349, 312), S(366, 296), S(350, 305), S(335, 313), S(326, 310), S(340, 302), S(330, 297), S(358, 303),
        S(325, 314), S(347, 304), S(330, 309), S(319, 311), S(313, 307), S(335, 294), S(337, 298), S(324, 299),
    ], [
        S(551, 529), S(531, 550), S(513, 559), S(522, 540), S(517, 534), S(512, 542), S(505, 549), S(516, 538),
        S(517, 543), S(517, 557), S(536, 555), S(519, 549), S(527, 542), S(514, 551), S(494, 558), S(492, 554),
        S(481, 554), S(535, 549), S(546, 547), S(548, 534), S(521, 536), S(519, 547), S(509, 548), S(492, 551),
        S(481, 542), S(488, 554), S(505, 553), S(518, 535), S(511, 533), S(488, 548), S(474, 552), S(471, 548),
        S(451, 526), S(470, 535), S(465, 541), S(485, 525), S(474, 529), S(463, 540), S(458, 540), S(456, 534),
        S(460, 509), S(476, 509), S(476, 518), S(480, 509), S(472, 514), S(457, 526), S(452, 527), S(450, 519),
        S(419, 519), S(477, 506), S(494, 505), S(495, 499), S(484, 502), S(478, 513), S(463, 513), S(456, 510),
        S(466, 497), S(463, 515), S(501, 506), S(508, 498), S(495, 504), S(489, 510), S(482, 504), S(481, 507),
    ], [
        S(1025, 974), S(1003, 984), S(1000, 1003), S(974, 1009), S(953, 1015), S(943, 1016), S(942, 1003), S(934, 990),
        S(998, 973), S(903, 1041), S(959, 1032), S(912, 1068), S(909, 1055), S(904, 1043), S(885, 1037), S(913, 1001),
        S(976, 1008), S(1000, 1004), S(1009, 1013), S(973, 1024), S(924, 1039), S(939, 1006), S(925, 998), S(944, 958),
        S(967, 994), S(945, 1026), S(941, 1040), S(946, 1027), S(939, 1019), S(924, 1009), S(928, 992), S(931, 969),
        S(948, 979), S(950, 987), S(939, 991), S(934, 999), S(930, 1001), S(938, 976), S(925, 970), S(936, 949),
        S(956, 917), S(959, 929), S(957, 946), S(951, 944), S(940, 954), S(937, 956), S(945, 931), S(933, 937),
        S(946, 882), S(969, 879), S(968, 886), S(958, 916), S(955, 918), S(953, 918), S(945, 924), S(941, 926),
        S(919, 878), S(909, 888), S(941, 879), S(958, 890), S(964, 895), S(945, 912), S(944, 908), S(942, 917),
    ], [
        S( 81, -48), S( 77,   0), S( 65,  -7), S( 80, -17), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S( 51,   0), S( 64,  30), S( 96,  19), S( 72,  13), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S( 11,  21), S( 67,  35), S( 74,  30), S( 83,  22), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-21,  13), S(  4,  32), S( 28,  32), S( 21,  34), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-73,   9), S(-53,  23), S(-46,  31), S(-63,  42), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(-73,   9), S(-29,   9), S(-65,  25), S(-69,  31), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(  1, -15), S(  4,  -6), S(-48,  11), S(-67,  15), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
        S(  3, -55), S( 20, -35), S(-31, -20), S(-10, -32), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    ],
];

const ROOK_SEMI_OPEN_FILE: [S; 8] = [S( 48,  21), S( 65,  -5), S( 32, -11), S( 17,   2), S( 25,   2), S( 18,  -2), S( 19,   2), S( 14,  15)];

const ROOK_FULL_OPEN_FILE: [S; 8] = [S( 60, -49), S( 39, -24), S( 30,  -8), S( 21,   0), S( 13,   4), S( 18,   5), S( 20,  -5), S( 23, -16)];

const ISOLATED_PAWN_FILE: [S; 8] = [S(-36,  -0), S( -2, -13), S(-14, -11), S(-20, -11), S(-16, -13), S(-14, -14), S( -9, -10), S( -5, -21)];

static PASSED_PAWN_PST: [S; 64] = [
    S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S(110,  42), S( 11,  64), S( 93,  40), S( 94,  31), S( 80,  31), S( 92,  35), S( 70,  48), S( 94,  50),
    S( 22,  98), S( 20,  91), S( 47,  73), S( 44,  36), S( 66,  37), S( 72,  54), S( 99,  62), S(108,  82),
    S( 15,  64), S( 36,  68), S( 16,  55), S( 10,  35), S( 29,  25), S( 41,  32), S( 54,  43), S( 52,  60),
    S( -6,  55), S(  6,  60), S( -1,  39), S( -2,  27), S(  5,  21), S(  2,  25), S( 29,  38), S( 32,  46),
    S(  4,  20), S( -9,  36), S(  4,  15), S(  7,  13), S( -7,   9), S( -4,  16), S(  2,  28), S( 14,  27),
    S(  4,  11), S(  8,  22), S( -4,  11), S( 20,  -3), S( -7,   3), S(-10,  20), S( -2,  30), S(  4,  22),
    S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
];

static MOBILITY: [S; 66] = [
    S(-77, -58), S(-27, -54), S( -1, -19), S( 18,   8), S( 33,  19), S( 36,  33),
    S( 50,  34), S( 61,  38), S( 73,  33), S(-61, -107), S(-34, -52), S( -0, -26),
    S(  9,   4), S( 25,  12), S( 38,  19), S( 46,  25), S( 52,  31), S( 58,  35),
    S( 63,  37), S( 68,  41), S( 82,  37), S( 96,  34), S(115,  35), S(-47, -36),
    S(-26, -12), S(-36,  12), S(-33,  40), S(-26,  48), S(-20,  55), S(-16,  62),
    S(-12,  68), S( -7,  70), S( -0,  73), S(  3,  77), S(  8,  78), S( 14,  80),
    S( 23,  75), S( 21,  79), S(  0,   0), S(  0,   0), S(-10,  -6), S( 11, -22),
    S(  8,   1), S( 38,  15), S( 41,  54), S( 40,  88), S( 45, 101), S( 50, 119),
    S( 56, 127), S( 63, 129), S( 70, 129), S( 74, 139), S( 78, 139), S( 78, 148),
    S( 81, 153), S( 81, 159), S( 87, 158), S( 93, 157), S( 97, 161), S(104, 159),
    S(121, 151), S(135, 143), S(141, 136), S(128, 143), S( 92, 118), S( 69,  90),
];

const BISHOP_PAIR: S = S( 36,  58);

static PAWN_DEFENDS: [S; 48] = [
    S(-66, -19), S(-37, -22), S(-11, -13), S( 15,  -1), S( 39,  15), S( 66,  25), S(108,  59), S(  0,   0),
    S( -4,  -7), S(  3,   5), S(  7,   7), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S( -3,  -4), S(  3,   2), S( -1,  17), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S( -7,  -5), S(  3,   5), S( 22,   4), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S(  2, -13), S( -5,  11), S( 13,   7), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
    S( 35, -12), S(-35,  12), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0), S(  0,   0),
];

const PHALANX_PAWNS: [S; 8] = [S(  0,   0), S( 76, 148), S(135, 142), S( 68,  57), S( 40,  17), S( 20,   6), S(  3,   2), S(  0,   0)];
