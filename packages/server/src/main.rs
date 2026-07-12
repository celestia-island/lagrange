//! `lagrange-server` binary — serve the comment backend or bootstrap an admin.
//!
//! Two subcommands:
//! - `serve` — bind the HTTP server (default `0.0.0.0:8080`).
//! - `create-admin` — add a moderator account (interactive or via flags), so
//!   the admin is self-controlled rather than hardcoded.

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;

use lagrange_server::{app, store::AppState};

#[derive(Parser)]
#[command(
    name = "lagrange-server",
    version,
    about = "Self-hostable lagrange comment backend"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the HTTP comment API.
    Serve {
        /// Bind address. Default `0.0.0.0:8080`.
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
        /// SQLite database path. Default `lagrange-comments.db`.
        #[arg(long, default_value = "lagrange-comments.db")]
        db: String,
        /// JWT secret. If unset, reads `LAGRANGE_JWT_SECRET` or generates a
        /// random one (logged once at startup).
        #[arg(long)]
        jwt_secret: Option<String>,
    },
    /// Create a moderator account.
    CreateAdmin {
        /// SQLite database path (must match `serve`).
        #[arg(long, default_value = "lagrange-comments.db")]
        db: String,
        /// JWT secret (must match `serve`).
        #[arg(long)]
        jwt_secret: Option<String>,
        /// Account name. If omitted, prompted interactively.
        #[arg(long)]
        name: Option<String>,
        /// Password. If omitted, prompted interactively (and confirmed).
        #[arg(long)]
        password: Option<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("lagrange_server=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Serve {
            bind,
            db,
            jwt_secret,
        } => serve(bind, db, jwt_secret),
        Command::CreateAdmin {
            db,
            jwt_secret,
            name,
            password,
        } => create_admin(db, jwt_secret, name, password),
    }
}

fn resolve_secret(secret: Option<String>) -> Result<Vec<u8>> {
    if let Some(s) = secret {
        return Ok(s.into_bytes());
    }
    if let Ok(env) = std::env::var("LAGRANGE_JWT_SECRET") {
        return Ok(env.into_bytes());
    }
    // Generate a random 32-byte secret and warn — it won't survive a restart,
    // invalidating all tokens, which is the safe default for a first run.
    let mut bytes = Vec::with_capacity(32);
    for _ in 0..2 {
        // ULID is 128 bits of time + randomness; two of them give 256 bits.
        let id = ulid::Ulid::new().to_string();
        bytes.extend_from_slice(id.as_bytes());
    }
    bytes.truncate(32);
    tracing::warn!(
        "no JWT secret configured; generated a random one. \
         Set --jwt-secret or LAGRANGE_JWT_SECRET for tokens to persist across restarts."
    );
    Ok(bytes)
}

fn serve(bind: String, db: String, jwt_secret: Option<String>) -> Result<()> {
    let secret = resolve_secret(jwt_secret)?;
    let state = AppState::open(&db, &secret).context("open database")?;
    let app = app(state);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&bind)
            .await
            .with_context(|| format!("bind {bind}"))?;
        info!(
            "lagrange-server listening on http://{bind}  (protocol {})",
            lagrange_protocol::PROTOCOL_VERSION
        );
        axum::serve(listener, app).await.context("serve")
    })?;
    Ok(())
}

fn create_admin(
    db: String,
    jwt_secret: Option<String>,
    name: Option<String>,
    password: Option<String>,
) -> Result<()> {
    let secret = resolve_secret(jwt_secret)?;
    let state = AppState::open(&db, &secret).context("open database")?;

    let name = match name {
        Some(n) => n,
        None => prompt("Admin username: ")?,
    };
    let password = match password {
        Some(p) => p,
        None => {
            let p1 = prompt_secret("Password: ")?;
            let p2 = prompt_secret("Confirm:    ")?;
            if p1 != p2 {
                anyhow::bail!("passwords did not match");
            }
            p1
        }
    };
    if password.len() < 8 {
        anyhow::bail!("password must be at least 8 characters");
    }

    let hash =
        lagrange_server::auth::hash_password(&password).map_err(|e| anyhow::anyhow!("{e}"))?;
    let account = state
        .create_account(&name, &hash, true)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    info!(
        "created moderator account '{}' (id={})",
        account.name, account.id
    );
    println!("✓ moderator '{}' created", account.name);
    Ok(())
}

fn prompt(msg: &str) -> Result<String> {
    let mut stdout = io::stdout();
    stdout.write_all(msg.as_bytes())?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("input required");
    }
    Ok(trimmed)
}

fn prompt_secret(msg: &str) -> Result<String> {
    let password = rpassword::prompt_password(msg).context("read password")?;
    if password.is_empty() {
        anyhow::bail!("password required");
    }
    Ok(password)
}
