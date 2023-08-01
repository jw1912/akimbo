mod core;
mod tuner;

use std::{env::args, io::stdin};
use tuner::{Data, gd_tune};
use crate::core::{Params, S};

fn main() -> std::io::Result<()> {
    let file_name = args().nth(1).unwrap_or(String::from("lichess.book"));
    // initialise data
    let mut data = Data::default();
    data.1 = 1;
    let num = data.add_contents(&file_name);
    println!("positions {num:.0}");

    // provide starting parameters
    let mut params = Params::default();
    let vals = [100., 300., 300., 500., 900.];
    for pc in 0..5 {
        for sq in 0..64 {
            params[64 * pc + sq] = S::new(vals[pc as usize]);
        }
    }

    let timer = std::time::Instant::now();
    data.error(0.09, &params);
    println!("time {}ms", timer.elapsed().as_millis());

    // carry out tuning
    gd_tune(&data, &mut params, 100000, 0.05, 1.);

    params.write_to_bin("../../resources/new_weights.bin")?;

    // wait for exit
    stdin().read_line(&mut String::new())?;
    Ok(())
}
