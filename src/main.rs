use clap::Parser;
use lagrange_library::cli::{self, Cli};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("lagrange_library=info")),
        )
        .with_target(true)
        .with_level(true)
        .with_timer(UtcTime::rfc_3339())
        .init();

    cli::run(Cli::parse())
}
