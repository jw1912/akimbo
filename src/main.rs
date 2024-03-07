mod attacks;
mod consts;
mod frc;
mod moves;
mod network;
mod position;
mod search;
mod tables;
mod thread;
mod util;

#[cfg(feature = "datagen")]
mod datagen;

#[cfg(not(feature = "datagen"))]
mod uci;

fn main() {
    println!("akimbo, created by Jamie Whiting");

    #[cfg(feature = "datagen")]
    {
        let threads = std::env::args().nth(1).unwrap().parse().unwrap();
        datagen::run_datagen(threads, None, None);
    }

    #[cfg(not(feature = "datagen"))]
    uci::run_uci();
}
