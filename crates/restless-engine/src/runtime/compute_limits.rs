//! Owner-configured container ceilings. Company config owns desired values;
//! Docker owns enforcement and the observed applied values. The host pool stays authoritative.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerLimits {
    pub cpu_millis: u32,
    pub memory_mib: u32,
}
impl ContainerLimits {
    pub(super) fn validate_values(self) -> Result<()> {
        anyhow::ensure!(
            (100..=256_000).contains(&self.cpu_millis),
            "CPU must be at least 0.1 cores."
        );
        anyhow::ensure!(
            (256..=1_048_576).contains(&self.memory_mib),
            "Memory must be at least 256 MB."
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyComputeLimits {
    pub runtime: Option<ContainerLimits>,
    pub computer: Option<ContainerLimits>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeTarget {
    Runtime,
    Computer,
}

impl ComputeTarget {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "runtime" => Ok(Self::Runtime),
            "computer" => Ok(Self::Computer),
            _ => bail!("Choose the work computer or browser computer."),
        }
    }

    pub fn saved(self, config: &CompanyConfig) -> Option<ContainerLimits> {
        config
            .compute_limits
            .as_ref()
            .and_then(|limits| match self {
                Self::Runtime => limits.runtime,
                Self::Computer => limits.computer,
            })
    }

    pub fn set(self, config: &mut CompanyConfig, value: Option<ContainerLimits>) {
        let limits = config.compute_limits.get_or_insert_with(Default::default);
        match self {
            Self::Runtime => limits.runtime = value,
            Self::Computer => limits.computer = value,
        }
    }

    fn container(self, company: &str) -> String {
        match self {
            Self::Runtime => container_name(company),
            Self::Computer => computer_container_name(company),
        }
    }

    pub fn validate(self, limits: ContainerLimits) -> Result<()> {
        let defaults = maximum(self);
        limits.validate_values()?;
        anyhow::ensure!(
            limits.memory_mib >= minimum_memory(self, defaults.memory_mib),
            "This computer needs at least {} MB of memory.",
            minimum_memory(self, defaults.memory_mib)
        );
        if let Some(max) = defaults.cpu_millis {
            anyhow::ensure!(
                limits.cpu_millis <= max,
                "CPU exceeds the capacity available for this computer."
            );
        }
        if let Some(max) = defaults.memory_mib {
            anyhow::ensure!(
                limits.memory_mib <= max,
                "Memory exceeds the capacity available for this computer."
            );
        }
        Ok(())
    }
}

fn minimum_memory(target: ComputeTarget, maximum: Option<u32>) -> u32 {
    // A managed turn reserves 512 MiB in addition to the existing computer.
    // 768 MiB is the smallest Runtime verified with a productive managed turn.
    let minimum = if target == ComputeTarget::Runtime {
        768
    } else {
        256
    };
    maximum.map_or(minimum, |provided| provided.min(minimum))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EffectiveLimits {
    pub cpu_millis: Option<u32>,
    pub memory_mib: Option<u32>,
}

fn defaults(target: ComputeTarget) -> EffectiveLimits {
    let memory = match target {
        ComputeTarget::Runtime => company_memory(),
        ComputeTarget::Computer => computer_memory(),
    };
    EffectiveLimits {
        cpu_millis: company_cpus()
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| (value * 1000.0).round() as u32),
        memory_mib: memory
            .and_then(|value| memory_bytes(&value))
            .map(|value| (value / (1024 * 1024)) as u32),
    }
}

fn effective(config: &CompanyConfig, target: ComputeTarget) -> EffectiveLimits {
    target.saved(config).map_or_else(
        || defaults(target),
        |saved| fit(Some(saved), maximum(target)),
    )
}

fn maximum(target: ComputeTarget) -> EffectiveLimits {
    let defaults = defaults(target);
    EffectiveLimits {
        cpu_millis: defaults.cpu_millis.or_else(|| {
            let cpus = std::thread::available_parallelism().map_or(1, usize::from);
            Some(cpus.saturating_sub(1).max(1) as u32 * 1000)
        }),
        memory_mib: defaults
            .memory_mib
            .or_else(|| host_memory_bytes().map(|bytes| (bytes / 4 * 3 / (1024 * 1024)) as u32)),
    }
}

fn fit(saved: Option<ContainerLimits>, maximum: EffectiveLimits) -> EffectiveLimits {
    match saved {
        None => maximum,
        Some(saved) => EffectiveLimits {
            cpu_millis: Some(
                maximum
                    .cpu_millis
                    .map_or(saved.cpu_millis, |max| saved.cpu_millis.min(max)),
            ),
            memory_mib: Some(
                maximum
                    .memory_mib
                    .map_or(saved.memory_mib, |max| saved.memory_mib.min(max)),
            ),
        },
    }
}

pub(super) fn bounds(
    config: &CompanyConfig,
    target: ComputeTarget,
) -> (Option<String>, Option<String>) {
    let limits = effective(config, target);
    (
        limits
            .cpu_millis
            .map(|value| format!("{:.3}", f64::from(value) / 1000.0)),
        limits.memory_mib.map(|value| format!("{value}m")),
    )
}

#[derive(Debug, Deserialize)]
struct Observed {
    cpu: u64,
    memory: u64,
    state: String,
}

async fn observe(name: &str) -> Result<Option<Observed>> {
    let out = docker_bounded(&["inspect", "-f",
        r#"{"cpu":{{.HostConfig.NanoCpus}},"memory":{{.HostConfig.Memory}},"state":{{json .State.Status}}}"#,
        name], Duration::from_secs(3)).await?;
    if !out.status.success() {
        let error = String::from_utf8_lossy(&out.stderr);
        anyhow::ensure!(
            error.contains("No such object") || error.contains("No such container"),
            "Computer limits could not be observed."
        );
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&out.stdout)?))
}

fn observed_limits(value: &Observed) -> EffectiveLimits {
    EffectiveLimits {
        cpu_millis: (value.cpu > 0).then_some((value.cpu / 1_000_000) as u32),
        memory_mib: (value.memory > 0).then_some((value.memory / (1024 * 1024)) as u32),
    }
}

#[derive(Serialize)]
pub struct TargetLimitsView {
    minimum_memory_mib: u32,
    saved: Option<ContainerLimits>,
    effective: EffectiveLimits,
    maximum: EffectiveLimits,
    applied: Option<EffectiveLimits>,
    state: String,
    pending: bool,
}

async fn target_view(config: &CompanyConfig, target: ComputeTarget) -> TargetLimitsView {
    let desired = effective(config, target);
    let (applied, state) = match observe(&target.container(&config.name)).await {
        Ok(Some(value)) => (Some(observed_limits(&value)), value.state),
        Ok(None) => (None, "absent".to_string()),
        Err(_) => (None, "unavailable".to_string()),
    };
    TargetLimitsView {
        minimum_memory_mib: minimum_memory(target, maximum(target).memory_mib),
        saved: target.saved(config),
        effective: desired,
        maximum: maximum(target),
        pending: applied.is_some_and(|value| value != desired),
        applied,
        state,
    }
}

#[derive(Serialize)]
pub struct ComputeLimitsView {
    runtime: TargetLimitsView,
    computer: TargetLimitsView,
}

pub async fn compute_limits_view(config: &CompanyConfig) -> ComputeLimitsView {
    let (runtime, computer) = tokio::join!(
        target_view(config, ComputeTarget::Runtime),
        target_view(config, ComputeTarget::Computer)
    );
    ComputeLimitsView { runtime, computer }
}

/// Apply a saved policy without starting a computer. Docker can refuse a RAM
/// reduction below current usage; keep desired state for normal maintenance to retry.
pub async fn apply_compute_limits(config: &CompanyConfig, target: ComputeTarget) -> Result<()> {
    let _operation = company_start_guard(&config.name).await;
    let current = CompanyConfig::load(&state_root(), &config.name)?;
    apply_locked(&current, target).await
}

pub(super) async fn apply_locked(config: &CompanyConfig, target: ComputeTarget) -> Result<()> {
    let name = target.container(&config.name);
    let Some(current) = observe(&name).await? else {
        return Ok(());
    };
    let desired = effective(config, target);
    if observed_limits(&current) == desired {
        return Ok(());
    }
    if current.state == "running"
        && desired.memory_mib.is_some_and(|wanted| {
            current.memory == 0 || u64::from(wanted) * 1024 * 1024 < current.memory
        })
    {
        let usage = docker_bounded(
            &["stats", "--no-stream", "--format", "{{.MemUsage}}", &name],
            Duration::from_secs(5),
        )
        .await?;
        anyhow::ensure!(
            usage.status.success(),
            "Current memory use is unavailable; limits remain pending."
        );
        let used = parse_memory_usage(&String::from_utf8_lossy(&usage.stdout))
            .context("Current memory use is unavailable; limits remain pending.")?;
        anyhow::ensure!(
            used.saturating_add(64 * 1024 * 1024)
                <= u64::from(desired.memory_mib.unwrap()) * 1024 * 1024,
            "Saved memory limit is waiting for current computer usage to fall."
        );
    }
    let (cpu, memory) = bounds(config, target);
    let mut args = vec![
        "update",
        "--cpus",
        cpu.as_deref().unwrap_or("0"),
        "--memory",
        memory.as_deref().unwrap_or("0"),
        "--memory-swap",
        memory.as_deref().unwrap_or("-1"),
    ];
    args.push(&name);
    let output = docker_bounded(&args, Duration::from_secs(5)).await?;
    anyhow::ensure!(
        output.status.success(),
        "Saved limits are waiting for current computer usage to fit."
    );
    Ok(())
}

fn parse_memory_usage(text: &str) -> Option<u64> {
    let value = text.split_whitespace().next()?;
    for (unit, scale) in [
        ("TiB", 1024_u64.pow(4)),
        ("GiB", 1024_u64.pow(3)),
        ("MiB", 1024_u64.pow(2)),
        ("KiB", 1024),
        ("B", 1),
    ] {
        if let Some(value) = value.strip_suffix(unit).and_then(|n| n.parse::<f64>().ok()) {
            return (value.is_finite() && value >= 0.0)
                .then_some((value * scale as f64).ceil() as u64);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_limits_never_expand_operator_capacity_and_refit_after_move() {
        let saved = ContainerLimits {
            cpu_millis: 2000,
            memory_mib: 2048,
        };
        assert_eq!(
            fit(
                Some(saved),
                EffectiveLimits {
                    cpu_millis: Some(500),
                    memory_mib: Some(768)
                }
            ),
            EffectiveLimits {
                cpu_millis: Some(500),
                memory_mib: Some(768)
            }
        );
        assert_eq!(
            fit(
                None,
                EffectiveLimits {
                    cpu_millis: None,
                    memory_mib: Some(4096)
                }
            ),
            EffectiveLimits {
                cpu_millis: None,
                memory_mib: Some(4096)
            }
        );
    }
    #[test]
    fn shrinking_memory_requires_a_known_conservative_usage_reading() {
        assert_eq!(parse_memory_usage("321.4MiB / 768MiB\n"), Some(337012327));
        assert_eq!(parse_memory_usage("1.5GiB / 4GiB"), Some(1610612736));
        assert_eq!(parse_memory_usage("-- / --"), None);
        assert_eq!(parse_memory_usage("NaNMiB / 4GiB"), None);
    }
    #[test]
    fn zero_limits_cannot_disable_docker_enforcement() {
        assert!(ContainerLimits {
            cpu_millis: 0,
            memory_mib: 768
        }
        .validate_values()
        .is_err());
        assert!(ContainerLimits {
            cpu_millis: 500,
            memory_mib: 0
        }
        .validate_values()
        .is_err());
        assert!(ContainerLimits {
            cpu_millis: 500,
            memory_mib: 768
        }
        .validate_values()
        .is_ok());
    }
    #[test]
    fn owner_memory_floor_preserves_room_for_a_turn_without_expanding_operator_limits() {
        assert_eq!(minimum_memory(ComputeTarget::Runtime, Some(4096)), 768);
        assert_eq!(minimum_memory(ComputeTarget::Runtime, Some(512)), 512);
        assert_eq!(minimum_memory(ComputeTarget::Computer, Some(2048)), 256);
    }
    #[test]
    fn old_configs_keep_operator_defaults_and_new_limits_round_trip() {
        let mut config: CompanyConfig = toml::from_str("name = 'limits_test'").unwrap();
        assert!(config.compute_limits.is_none());
        let limits = ContainerLimits {
            cpu_millis: 500,
            memory_mib: 768,
        };
        ComputeTarget::Runtime.set(&mut config, Some(limits));
        let restored: CompanyConfig = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(ComputeTarget::Runtime.saved(&restored), Some(limits));
        assert_eq!(ComputeTarget::Computer.saved(&restored), None);
    }
}
