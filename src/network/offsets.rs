use super::indices;

pub const PAWN: usize = 0;
pub const KNIGHT: usize = PAWN + 6 * indices::PAWN;
pub const BISHOP: usize = KNIGHT + 12 * indices::KNIGHT[64];
pub const ROOK: usize = BISHOP + 10 * indices::BISHOP[64];
pub const QUEEN: usize = ROOK + 10 * indices::ROOK[64];
pub const KING: usize = QUEEN + 12 * indices::QUEEN[64];
pub const END: usize = KING + 8 * indices::KING[64];
