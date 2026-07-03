use clap::Parser;
use lagrange_library::cli::{self, Cli};
use tracing_subscriber::EnvFilter;

struct Timer;

impl tracing_subscriber::fmt::time::FormatTime for Timer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("lagrange_library=info")),
        )
        .with_timer(Timer)
        .init();

    cli::run(Cli::parse())
}
