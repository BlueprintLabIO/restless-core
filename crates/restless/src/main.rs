//! restless — the owner-surface CLI. Sprint 01: lifecycle commands over the
//! restlessd unix socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "restless", about = "Restless owner surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bring a company environment up (create if absent, then start).
    Up { company: String },
    /// Stop a company environment. The volume — files, Git, browser — survives.
    Down { company: String },
    /// Company environment status.
    Status { company: String },
}

fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (cmd, company) = match cli.command {
        Command::Up { company } => ("up", company),
        Command::Down { company } => ("down", company),
        Command::Status { company } => ("status", company),
    };
    let request = serde_json::json!({ "cmd": cmd, "company": company });
    let response = request_once(&request.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&response).context("parse response")?;
    if parsed["ok"].as_bool() == Some(true) {
        println!("{}", parsed["data"].as_str().unwrap_or("ok"));
        Ok(())
    } else {
        bail!("{}", parsed["error"].as_str().unwrap_or("unknown error"))
    }
}

fn request_once(line: &str) -> Result<String> {
    let sock = state_root().join("restlessd.sock");
    let mut stream = UnixStream::connect(&sock)
        .with_context(|| format!("connect {} — is restlessd running?", sock.display()))?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
}
