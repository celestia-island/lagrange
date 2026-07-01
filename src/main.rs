use clap::Parser;
use lagrange_library::cli::{self, Cli};

fn main() -> anyhow::Result<()> {
    cli::run(Cli::parse())
}
