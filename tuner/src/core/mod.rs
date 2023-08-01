mod params;
mod position;
mod score;

pub use position::{Position, sigmoid, HITS};
pub use score::S;
pub use params::Params;

pub const NUM_PARAMS: usize = 5 * 64 * 64;
