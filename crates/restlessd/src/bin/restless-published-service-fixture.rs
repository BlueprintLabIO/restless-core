use std::path::PathBuf;

use anyhow::{bail, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args_os();
    let _binary = arguments.next();
    let Some(config_path) = arguments.next() else {
        bail!("usage: restless-published-service-fixture <absolute-config-path>");
    };
    if arguments.next().is_some() {
        bail!("fixture accepts exactly one config path");
    }
    restlessd::published_service_fixture::run_from_config_path(&PathBuf::from(config_path)).await
}
