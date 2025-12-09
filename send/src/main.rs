mod arp;
mod checksum;
mod cli;
mod config;
mod error;
mod ethernet;
mod icmp;
mod ipv4;
mod ipv4_addr;
mod mac;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::ethernet::{list_adapters, send_packets};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => list_adapters(),
        Command::Send(args) => send_packets(args),
    }
}
