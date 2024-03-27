mod attacks;
mod consts;
mod frc;
mod moves;
mod network;
mod position;
mod search;
mod tables;
mod thread;
mod uci;
mod util;

fn main() {
    println!("akimbo, created by Jamie Whiting");
    uci::run_uci();
}
