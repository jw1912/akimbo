mod params;
mod position;
mod score;

pub use position::{Position, sigmoid};
pub use score::S;
pub use params::Params;

pub const OFFSET: usize = 5 * 64 * 64;
pub const PASSER: usize = 2 * OFFSET;
pub const NUM_PARAMS: usize = PASSER + 64;
