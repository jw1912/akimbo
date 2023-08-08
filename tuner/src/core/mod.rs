mod params;
mod position;
mod score;

pub use position::{Position, sigmoid};
pub use score::S;
pub use params::Params;

pub const OFFSET: usize = 5 * 64 * 64;
pub const PASSER: usize = 2 * OFFSET;
pub const OPEN: usize = PASSER + 64;
pub const SEMI: usize = OPEN + 8;
pub const BLOCKED: usize = SEMI + 8;
pub const NUM_PARAMS: usize = BLOCKED + 8;
