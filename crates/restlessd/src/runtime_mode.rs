//! Identity verification and computer placement are separate deployment choices.
use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeMode {
    Local,
    Hosted,
}

impl RuntimeMode {
    pub(crate) fn from_env(network_entry: bool) -> Result<Self> {
        let configured = match std::env::var("RESTLESS_RUNTIME_MODE") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => return Err(error).context("read RESTLESS_RUNTIME_MODE"),
        };
        Self::parse(configured.as_deref(), network_entry)
    }

    fn parse(configured: Option<&str>, network_entry: bool) -> Result<Self> {
        match configured {
            // Preserve existing local and Cloud deployments unless explicitly changed.
            None if network_entry => Ok(Self::Hosted),
            None | Some("local") => Ok(Self::Local),
            Some("hosted") if network_entry => Ok(Self::Hosted),
            Some("hosted") => {
                anyhow::bail!("RESTLESS_RUNTIME_MODE=hosted requires RESTLESS_ENTRY_MODE=network")
            }
            Some(value) => {
                anyhow::bail!("RESTLESS_RUNTIME_MODE must be local or hosted, not {value:?}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_deployments_keep_their_runtime_placement() {
        assert_eq!(RuntimeMode::parse(None, false).unwrap(), RuntimeMode::Local);
        assert_eq!(RuntimeMode::parse(None, true).unwrap(), RuntimeMode::Hosted);
    }

    #[test]
    fn authenticated_self_hosting_can_use_local_computers() {
        assert_eq!(
            RuntimeMode::parse(Some("local"), true).unwrap(),
            RuntimeMode::Local
        );
        assert_eq!(
            RuntimeMode::parse(Some("hosted"), true).unwrap(),
            RuntimeMode::Hosted
        );
    }

    #[test]
    fn invalid_configuration_never_falls_back_to_a_different_boundary() {
        assert!(RuntimeMode::parse(Some("hosted"), false).is_err());
        for value in ["", "auto", "LOCAL", " local"] {
            assert!(RuntimeMode::parse(Some(value), true).is_err());
            assert!(RuntimeMode::parse(Some(value), false).is_err());
        }
    }
}
