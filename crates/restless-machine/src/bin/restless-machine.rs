//! `restless-machine`: lend this computer to your Restless plane (S99).
//!
//!   restless-machine connect <plane URL> <pairing code>   pair once, then stay linked
//!   restless-machine run                                  stay linked with the saved pairing
//!
//! It runs as a container with the Docker socket, started by the plane's install script. Nothing
//! listens on the machine; it opens one outbound link to the plane.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{bail, Context as _, Result};
use restless_machine::docker_filter::Scope;
use restless_machine::docker_proxy::DockerProxy;
use restless_machine::machine::{self, Paired};

fn state_path() -> PathBuf {
    std::env::var_os("RESTLESS_MACHINE_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/restless-machine/machine.json"))
}

fn docker_socket() -> PathBuf {
    std::env::var_os("RESTLESS_MACHINE_DOCKER_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/run/docker.sock"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let state = state_path();
    let socket = docker_socket();
    let paired = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["connect", plane, code] => match Paired::load(&state)? {
            Some(paired) if paired.plane == plane.trim_end_matches('/') => {
                tracing::info!(machine = %paired.label, "already paired");
                paired
            }
            _ => {
                let lookup = DockerProxy::new(socket.clone(), Arc::new(RwLock::new(Scope::default())));
                let facts = machine::facts(&lookup).await;
                if facts.docker_version.is_empty() {
                    bail!("Docker is not answering on {}", socket.display());
                }
                let paired = machine::pair(plane, code, facts).await?;
                paired.save(&state)?;
                println!("Connected as \"{}\".", paired.label);
                paired
            }
        },
        ["run"] | [] => Paired::load(&state)?.context("this computer is not paired; run connect with a code first")?,
        _ => bail!("usage: restless-machine connect <plane URL> <code> | run"),
    };
    machine::run(paired, socket, state).await
}
