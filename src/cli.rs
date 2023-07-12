use std::fmt::{Display, Formatter};

use clap::{Parser, Subcommand};

use crate::block_chain::BlockChain;
use crate::tools::{bytes2hex, hash2str};
use crate::transaction::{TXInput, TXOutput};
use crate::wallet::Wallets;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

pub fn run_cmd() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::MineBlock { data }) => {
            // block_chain.mine_block(data);
        }
        Some(Commands::PrintChain) => {
            match BlockChain::get() {
                Some(mut block_chain) => {
                    block_chain.print_chain();
                }
                None => println!("Database not exits"),
            }
            // block_chain.print_chain();
        }
        Some(Commands::CreateChain { address }) => {
            BlockChain::create(String::from(address));
        }
        Some(Commands::Send { from, to, amount }) => match BlockChain::get() {
            Some(mut block_chain) => {
                println!("Send {} from {} to {}", amount, from, to);
                match block_chain.new_tx(from.as_str(), to.as_str(), *amount) {
                    Ok(tx) => {
                        println!("Create transaction");
                        block_chain.mine_block(vec![tx]);
                        println!("Mining block success");
                    }
                    Err(err) => {
                        println!("{}", err);
                    }
                }
            }
            None => println!("Database not exits"),
        },
        Some(Commands::Balance { address }) => {
            let mut block_chain = match BlockChain::get() {
                Some(block_chain) => block_chain,
                None => {
                    println!("Database not exits");
                    return;
                }
            };
            // let pub_key_hash = extract_pub_key_hash(address);
            // for block in BlockChainIter::new(&mut block_chain) {
            //     for tx in block.transactions {
            //         let is_spent = match tx.v_in.get(0) {
            //             Some(input) => input.use_key(&pub_key_hash),
            //             None => false,
            //         };
            //         let is_recv = match tx.v_out.get(0) {
            //             Some(output) => output.is_locking_with_key(&pub_key_hash),
            //             None => false,
            //         };
            //
            //         if is_spent || is_recv {
            //             println!("transaction: {}", hash2str(&tx.id));
            //             println!("inputs:");
            //             for input in tx.v_in {
            //                 println!("{}", input);
            //             }
            //             println!("outputs:");
            //             for out in tx.v_out {
            //                 println!("{}", out)
            //             }
            //             println!();
            //         }
            //     }
            // }

            println!(
                "Balance of {}: {}",
                address,
                block_chain.get_balance(address)
            );
        }
        Some(Commands::CreateWallet) => {
            let mut wallets = Wallets::new();
            let address = wallets.create_wallet();
            println!("Your address is: {}", address);
        }
        None => {}
    }
}

impl Display for TXInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tx_id: {}\nv_out_idx: {}\npublic key hash: {} \nsignature: {}",
            match self.tx_id {
                Some(hash) => hash2str(&hash),
                None => String::from("None"),
            },
            match self.v_out_idx {
                Some(idx) => idx.to_string(),
                None => String::from("None"),
            },
            bytes2hex(self.pub_key.as_slice()),
            match &self.signature {
                Some(bytes) => bytes2hex(bytes.as_slice()),
                None => String::from("None"),
            }
        )
    }
}

impl Display for TXOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "value: {}\nscript_pub_key: {}",
            self.value,
            bytes2hex(self.pub_key_hash.as_slice()),
        )
    }
}

#[derive(Subcommand)]
enum Commands {
    CreateChain {
        address: String,
    },
    MineBlock {
        data: String,
    },
    Send {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
    Balance {
        address: String,
    },
    PrintChain,
    CreateWallet,
}
