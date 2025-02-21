use super::attacks;

macro_rules! init_add_assign {
    (|$sq:ident, $init:expr, $size:literal | $($rest:tt)+) => {{
        let mut $sq = 0;
        let mut res = [{$($rest)+}; $size + 1];
        let mut val = $init;
        while $sq < $size {
            res[$sq] = val;
            val += {$($rest)+};
            $sq += 1;
        }

        res[$size] = val;

        res
    }};
}

pub const PAWN: usize = 84;
pub const KNIGHT: [usize; 65] =
    init_add_assign!(|sq, 0, 64| attacks::KNIGHT[sq].count_ones() as usize);
pub const BISHOP: [usize; 65] =
    init_add_assign!(|sq, 0, 64| attacks::BISHOP[sq].count_ones() as usize);
pub const ROOK: [usize; 65] =
    init_add_assign!(|sq, 0, 64| attacks::ROOK[sq].count_ones() as usize);
pub const QUEEN: [usize; 65] =
    init_add_assign!(|sq, 0, 64| attacks::QUEEN[sq].count_ones() as usize);
pub const KING: [usize; 65] =
    init_add_assign!(|sq, 0, 64| attacks::KING[sq].count_ones() as usize);
