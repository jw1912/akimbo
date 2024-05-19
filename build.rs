use std::env;

const DEFAULT_PATH: &str = "resources/net.bin";

fn main() {
    println!("cargo:rerun-if-env-changed=EVALFILE");
    println!("cargo:rerun-if-changed=resources/net.bin");
    let net_path = env::var("EVALFILE").unwrap_or(DEFAULT_PATH.into());
    if net_path != DEFAULT_PATH {
        std::fs::copy(net_path, DEFAULT_PATH).unwrap();
    }
}
