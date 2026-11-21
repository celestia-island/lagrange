use clap::Parser;
use lagrange_library::cli::{self, Cli};
use tracing_subscriber::EnvFilter;

struct Timer;

impl tracing_subscriber::fmt::time::FormatTime for Timer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(
            w,
            "{}",
            jiff::ZonedDateTime::now().strftime("%Y-%m-%d %H:%M:%S")
        )
    }
}

fn main() -> anyhow::Result<()> {
    // Logs go to STDERR unconditionally: `lagrange dev --mcp` serves its
    // MCP protocol on stdout (one JSON-RPC line per message), and a single
    // stray log line there would corrupt the client's stream. stderr is
    // also where CLI logs belong conventionally.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("lagrange_library=info")),
        )
        .with_timer(Timer)
        .init();

    cli::run(Cli::parse())
}
