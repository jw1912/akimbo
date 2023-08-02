mod util;
mod thread;

use thread::ThreadData;

fn main() {
    let mut thread = ThreadData::new(40_000, 8);

    thread.run_datagen(140);
}
