use crate::{c_enum, init};

c_enum!(u8, Bound,
    LOWER = 0,
    EXACT = 1,
    UPPER = 2
);

c_enum!(i32, Score,
    MAX = 30000,
    MATE = Self::MAX - 256,
    DRAW = 0
);

c_enum!(i32, MoveScore,
    HASH = 3000000,
    HISTORY_MAX = 16384,
    PROMO = 70000,
    KILLER = 69000,
    CAPTURE = 8 * Self::HISTORY_MAX
);

c_enum!(usize, Side,
    WHITE = 0,
    BLACK = 1
);

c_enum!(usize, Piece,
    EMPTY = 0,
    PAWN = 2,
    KNIGHT = 3,
    BISHOP = 4,
    ROOK = 5,
    QUEEN = 6,
    KING = 7
);

c_enum!(u8, Flag,
    QUIET = 0,
    DBL = 1,
    KS = 2,
    QS = 3,
    CAP = 4,
    ENP = 5,
    PROMO = 8,
    QPR = 11,
    NPC = 12,
    QPC = 15
);

c_enum!(u8, Rights,
    WQS = 8,
    WKS = 4,
    BQS = 2,
    BKS = 1,
    WHITE = Self::WQS | Self::WKS,
    BLACK = Self::BQS | Self::BKS
);

c_enum!([u64; 2], Rank,
    PEN = [0xFF000000000000, 0xFF00],
    DBL = [0xFF000000, 0xFF00000000]
);

c_enum!(u64, File,
    A = 0x101010101010101,
    H = Self::A << 7
);

pub const CASTLE_MASK: [u8; 64] = init!(|sq, 64|
    match sq {
        0 => 7,
        4 => 3,
        7 => 11,
        56 => 13,
        60 => 12,
        63 => 14,
        _ => 15,
    }
);

pub const ROOK_MOVES: [[(usize, usize); 2]; 2] = [[(0, 3), (56, 59)], [(7, 5), (63, 61)]];

const fn rand(mut seed: u64) -> u64 {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

pub struct ZobristVals {
    pub pcs: [[[u64; 64]; 8]; 2],
    pub cr: [u64; 16],
    pub enp: [u64; 8],
    pub c: [u64; 2],
}

pub static ZVALS: ZobristVals = {
    let mut seed = 180_620_142;
    seed = rand(seed);

    let c = [0, seed];

    let pcs = init!(|side, 2| init!(|pc, 8|
        init!(|sq, 64| {
            if pc < 2 {
                0
            } else {
                seed = rand(seed);
                seed
            }
        })
    ));

    let cf = init!(|i, 4| {
        seed = rand(seed);
        seed
    });

    let cr = init!(|i, 16| {
          ((i & 1 > 0) as u64 * cf[0]) ^ ((i & 2 > 0) as u64 * cf[1])
        ^ ((i & 4 > 0) as u64 * cf[2]) ^ ((i & 8 > 0) as u64 * cf[3])
    });

    let enp = init!(|i, 8| {
        seed = rand(seed);
        seed
    });

    ZobristVals { pcs, cr, enp, c }
};

pub const SEE_VALS: [i32; 8] = [0, 0, 100, 450, 450, 650, 1250, 0];

const FRONT_SPANS: [u64; 64] = init!(|sq, 64| {
    let mut bb = (1 << sq) << 8;
    bb |= bb << 8;
    bb |= bb << 16;
    bb |= bb << 32;
    bb | (bb & !File::H) << 1 | (bb & !File::A) >> 1
});

pub const SPANS: [[u64; 64]; 2] = [
    FRONT_SPANS,
    init!(|sq, 64| FRONT_SPANS[sq ^ 56].swap_bytes()),
];

pub const SIDE: [i32; 2] = [1, -1];

pub const PHASE_VALS: [i32; 8] = [0, 0, 0, 1, 1, 2, 4, 0];