mod attacks;
mod consts;
mod eval;
mod frc;
mod moves;
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
