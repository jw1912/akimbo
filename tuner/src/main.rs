mod core;
mod tuner;

use std::env::args;
use tuner::{Data, gd_tune};
use crate::core::{Params, S};

fn main() -> std::io::Result<()> {
    let file_name = args().nth(1).unwrap_or(String::from("resources/wha.epd"));

    // initialise data
    let mut data = Data::default();
    data.1 = 4;
    data.add_contents::<false>(&file_name);

    // provide starting parameters
    let mut params = Params::default();
    let vals = [100., 300., 300., 500., 900.];
    for pc in 0..5 {
        for ksq in 0..64 {
            for sq in 0..64 {
                params[5 * 64 * ksq + 64 * pc + sq] = S::new(vals[pc as usize]);
            }
        }
    }

    // carry out tuning
    gd_tune(&data, &mut params, 5000, 0.05, 1.);

    params.write_to_bin("resources/new_weights.bin")?;

    // exit
    Ok(())
}
