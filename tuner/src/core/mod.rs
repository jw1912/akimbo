mod params;
mod position;
mod score;

pub use position::{Position, sigmoid, HITS};
pub use score::S;
pub use params::Params;

pub const OFFSET: usize = 5 * 64 * 64;
pub const NUM_PARAMS: usize = 2 * OFFSET;
