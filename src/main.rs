extern crate core;

use crate::cli::run_cmd;
mod block;
mod block_chain;
mod cli;
mod pow;
mod tools;
mod transaction;
mod wallet;

fn main() {
    run_cmd();
}
