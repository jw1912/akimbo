mod data;

use std::time::Instant;
pub use data::Data;
use crate::core::{Params, NUM_PARAMS};

fn optimise_k(params: &Params, data: &Data) -> f64 {
    let mut k = 0.009;
    let delta = 0.00001;
    let goal = 0.000001;
    let mut dev = 1f64;
    while dev.abs() > goal {
        let right = data.error(k + delta, params);
        let left = data.error(k - delta, params);
        dev = (right - left) / (500. * delta);
        println!("k {k:.4} decr {left:.5} incr {right:.5}");
        k -= dev;
    }
    let error = data.error(k, params);
    println!("k {k:.6} error {error:.5}");
    k
}

pub fn gd_tune(data: &Data, params: &mut Params, max_epochs: usize, mut rate: f64, decr: f64) {
    let k = optimise_k(params, data);
    let mut velocity = Params::default();
    let mut momentum = Params::default();
    let mut error = data.error(k, params);
    let mut old;
    let goal = 0.000005;
    let b1 = 0.9;
    let b2 = 0.999;

    let timer = Instant::now();

    for epoch in 1..=max_epochs {
        let gradients = data.gradients(k, params);
        for i in 0..NUM_PARAMS as u16 {
            let adj = (-2. * k / data.num()) * gradients[i];
            momentum[i] = b1 * momentum[i] + (1. - b1) * adj;
            velocity[i] = b2 * velocity[i] + (1. - b2) * adj * adj;
            params[i] -= rate * momentum[i] / (velocity[i].sqrt() + 0.00000001);
        }

        if epoch % 100 == 0 {
            rate *= decr;
            old = error;
            error = data.error(k, params);
            let eps = epoch as f64 / timer.elapsed().as_secs_f64();
            println!("epoch {epoch} error {error:.5} rate {rate:.3} eps {eps:.2}/sec");
            if old - error < goal {
                break
            }
        }
    }
}