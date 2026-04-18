use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

const TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const VULKAN_STATE_FILE: &str = "/run/tuff-vulkan-state";
const RADV_PERFTEST: &str = "aco,nv_ms";

#[derive(Parser)]
#[command(author, version, about = "TUFF distro helper utilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    RenderPackageList(RenderPackageListArgs),
    WriteReleaseManifest(WriteReleaseManifestArgs),
    VulkanInit(VulkanInitArgs),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Lines,
    Comma,
}

#[derive(Parser)]
struct RenderPackageListArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Lines)]
    format: OutputFormat,
    #[arg(long)]
    distro_dir: Option<PathBuf>,
    groups: Vec<String>,
}

#[derive(Parser)]
struct WriteReleaseManifestArgs {
    #[arg(long)]
    distro_dir: Option<PathBuf>,
    #[arg(long)]
    vm_image: Option<PathBuf>,
    #[arg(long)]
    release_dir: Option<PathBuf>,
    #[arg(long)]
    timestamp: Option<String>,
    #[arg(long, default_value = "bootstrap")]
    channel: String,
    #[arg(long, default_value = "v1-bootstrap")]
    version: String,
    #[arg(long, default_value = "Debian 13 (Trixie)")]
    base: String,
    #[arg(long, default_value = "tuff-base")]
    package_group: String,
}

#[derive(Parser)]
struct VulkanInitArgs {
    #[arg(long, default_value = VULKAN_STATE_FILE)]
    state_file: PathBuf,
}

#[derive(Serialize)]
struct ReleaseManifest {
    project: &'static str,
    component: &'static str,
    version: String,
    timestamp: String,
    channel: String,
    artifacts: Vec<Artifact>,
    base: String,
}

#[derive(Serialize)]
struct Artifact {
    name: String,
    #[serde(rename = "type")]
    artifact_type: &'static str,
    arch: &'static str,
    sha256: String,
}

#[derive(Debug)]
struct VulkanProbe {
    offload_enabled: bool,
    status: String,
    device_count: usize,
    render_node_count: usize,
    device_names: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::RenderPackageList(args) => run_render_package_list(args),
        Commands::WriteReleaseManifest(args) => run_write_release_manifest(args),
        Commands::VulkanInit(args) => run_vulkan_init(args),
    }
}

fn run_render_package_list(args: RenderPackageListArgs) -> Result<()> {
    let distro_dir = args.distro_dir.unwrap_or_else(default_distro_dir);
    let groups = if args.groups.is_empty() {
        vec!["tuff-base".to_owned()]
    } else {
        args.groups
    };
    let packages = collect_packages(&distro_dir, &groups)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match args.format {
        OutputFormat::Lines => {
            for package in packages {
                writeln!(out, "{package}")?;
            }
        }
        OutputFormat::Comma => {
            writeln!(out, "{}", packages.join(","))?;
        }
    }
    Ok(())
}

fn run_write_release_manifest(args: WriteReleaseManifestArgs) -> Result<()> {
    let distro_dir = args.distro_dir.unwrap_or_else(default_distro_dir);
    let vm_image = args
        .vm_image
        .unwrap_or_else(|| distro_dir.join("out/images/vm/tuff-vm-stable-amd64-minbase.raw"));
    if !vm_image.is_file() {
        bail!("VM image not found: {}", vm_image.display());
    }

    let timestamp = match args.timestamp {
        Some(ts) => ts,
        None => OffsetDateTime::now_utc()
            .format(TIMESTAMP_FORMAT)
            .context("failed to format UTC timestamp")?,
    };
    let release_dir = args
        .release_dir
        .unwrap_or_else(|| distro_dir.join("out/release").join(&timestamp));
    fs::create_dir_all(&release_dir)
        .with_context(|| format!("failed to create release directory {}", release_dir.display()))?;

    let artifact_name = vm_image
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("failed to derive artifact name from {}", vm_image.display()))?
        .to_owned();
    let artifact_path = release_dir.join(&artifact_name);
    fs::copy(&vm_image, &artifact_path).with_context(|| {
        format!(
            "failed to copy VM image from {} to {}",
            vm_image.display(),
            artifact_path.display()
        )
    })?;

    let sha256 = sha256_hex(&vm_image)?;
    fs::write(
        release_dir.join("SHA256SUMS"),
        format!("{sha256}  {artifact_name}\n"),
    )
    .with_context(|| format!("failed to write SHA256SUMS into {}", release_dir.display()))?;

    let packages = collect_packages(&distro_dir, &[args.package_group.clone()])?;
    fs::write(release_dir.join("tuff-base.manifest"), packages.join("\n") + "\n")
        .with_context(|| format!("failed to write package manifest into {}", release_dir.display()))?;

    let manifest = ReleaseManifest {
        project: "TUFF-RADICAL",
        component: "distro-bootstrap",
        version: args.version,
        timestamp,
        channel: args.channel,
        artifacts: vec![Artifact {
            name: artifact_name,
            artifact_type: "raw-vm-image",
            arch: "amd64",
            sha256,
        }],
        base: args.base,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(release_dir.join("manifest.json"), manifest_json + "\n")
        .with_context(|| format!("failed to write manifest.json into {}", release_dir.display()))?;

    println!("--- Release Manifest Written: {} ---", release_dir.display());
    Ok(())
}

fn run_vulkan_init(args: VulkanInitArgs) -> Result<()> {
    println!("--- TUFF-RADICAL [VULKAN-01]: Initializing High-Performance Compute Domain ---");

    if command_exists("cpupower") {
        println!("[INFO] Forcing performance CPU governor for maximum AVX/SIMD throughput.");
        let _ = Command::new("cpupower")
            .args(["frequency-set", "-g", "performance"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    let probe = probe_vulkan()?;
    write_vulkan_state(&args.state_file, &probe)?;

    match probe.status.as_str() {
        "ready" => println!(
            "[SUCCESS] Vulkan ACTIVE. devices={} render-nodes={} names={}",
            probe.device_count,
            probe.render_node_count,
            probe.device_names.join(", ")
        ),
        "missing-vulkaninfo" => println!(
            "[WARN] vulkaninfo is missing. render-nodes={} GPU offload disabled.",
            probe.render_node_count
        ),
        "no-vulkan-device" => println!(
            "[INFO] No Vulkan-compatible GPU detected. render-nodes={} CPU fallback remains active.",
            probe.render_node_count
        ),
        "no-render-node" => println!(
            "[WARN] Vulkan reported {} device(s), but /dev/dri/renderD* is missing.",
            probe.device_count
        ),
        "vulkaninfo-failed" => println!(
            "[WARN] vulkaninfo failed after detecting {} render node(s). GPU offload disabled.",
            probe.render_node_count
        ),
        _ => println!(
            "[WARN] Vulkan probe ended in state={} devices={} render-nodes={}.",
            probe.status, probe.device_count, probe.render_node_count
        ),
    }

    Ok(())
}

fn probe_vulkan() -> Result<VulkanProbe> {
    let render_node_count = count_render_nodes(Path::new("/dev/dri"))?;
    if !command_exists("vulkaninfo") {
        return Ok(VulkanProbe {
            offload_enabled: false,
            status: "missing-vulkaninfo".to_owned(),
            device_count: 0,
            render_node_count,
            device_names: Vec::new(),
        });
    }

    let output = Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .context("failed to execute vulkaninfo --summary")?;
    let summary = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Ok(VulkanProbe {
            offload_enabled: false,
            status: "vulkaninfo-failed".to_owned(),
            device_count: 0,
            render_node_count,
            device_names: Vec::new(),
        });
    }

    let physical_devices = parse_physical_devices(&summary);
    let device_names = physical_devices
        .iter()
        .map(|device| device.name.clone())
        .collect::<Vec<_>>();
    let device_count = physical_devices.len();

    let status = if device_count == 0 {
        "no-vulkan-device"
    } else if render_node_count == 0 {
        "no-render-node"
    } else {
        "ready"
    };

    Ok(VulkanProbe {
        offload_enabled: status == "ready",
        status: status.to_owned(),
        device_count,
        render_node_count,
        device_names,
    })
}

fn write_vulkan_state(path: &Path, probe: &VulkanProbe) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let device_names = if probe.device_names.is_empty() {
        "none".to_owned()
    } else {
        probe.device_names.join(";")
    };
    let content = format!(
        "TUFF_VULKAN_OFFLOAD={}\n\
TUFF_INTEL_COMPUTE_ACTIVE={}\n\
TUFF_VULKAN_STATUS={}\n\
TUFF_VULKAN_DEVICE_COUNT={}\n\
TUFF_VULKAN_RENDER_NODE_COUNT={}\n\
TUFF_VULKAN_DEVICE_NAMES={}\n\
RADV_PERFTEST={}\n",
        bool_as_int(probe.offload_enabled),
        bool_as_int(probe.offload_enabled),
        shell_quote(&probe.status),
        probe.device_count,
        probe.render_node_count,
        shell_quote(&device_names),
        shell_quote(RADV_PERFTEST),
    );

    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn collect_packages(distro_dir: &Path, groups: &[String]) -> Result<Vec<String>> {
    let mut packages = Vec::new();
    let mut seen = HashSet::new();

    for group in groups {
        let group_dir = distro_dir.join("packages").join(group);
        if !group_dir.is_dir() {
            bail!("missing package directory: {}", group_dir.display());
        }

        let mut files = fs::read_dir(&group_dir)
            .with_context(|| format!("failed to read {}", group_dir.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension() == Some(OsStr::new("txt")))
            .collect::<Vec<_>>();
        files.sort();

        if files.is_empty() {
            bail!("no package lists (*.txt) found in {}", group_dir.display());
        }

        for file in files {
            let contents = fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            for package in parse_package_lines(&contents) {
                if seen.insert(package.clone()) {
                    packages.push(package);
                }
            }
        }
    }

    Ok(packages)
}

fn parse_package_lines(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let candidate = line.split('#').next().unwrap_or("").trim();
            if candidate.is_empty() {
                None
            } else {
                Some(candidate.to_owned())
            }
        })
        .collect()
}

fn count_render_nodes(dev_dri: &Path) -> Result<usize> {
    if !dev_dri.exists() {
        return Ok(0);
    }

    let count = fs::read_dir(dev_dri)
        .with_context(|| format!("failed to read {}", dev_dri.display()))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("renderD"))
                .unwrap_or(false)
        })
        .count();

    Ok(count)
}

#[derive(Default)]
struct VulkanDeviceRecord {
    device_type: Option<String>,
    name: Option<String>,
}

#[derive(Clone)]
struct PhysicalDevice {
    name: String,
}

fn parse_physical_devices(summary: &str) -> Vec<PhysicalDevice> {
    let mut current = VulkanDeviceRecord::default();
    let mut devices = Vec::new();

    for line in summary.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GPU") && trimmed.ends_with(':') {
            push_physical_device(&mut devices, &current);
            current = VulkanDeviceRecord::default();
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "deviceType" => current.device_type = Some(value.to_owned()),
                "deviceName" => current.name = Some(value.to_owned()),
                _ => {}
            }
        }
    }

    push_physical_device(&mut devices, &current);
    devices
}

fn push_physical_device(devices: &mut Vec<PhysicalDevice>, record: &VulkanDeviceRecord) {
    let Some(device_type) = record.device_type.as_deref() else {
        return;
    };
    if device_type != "PHYSICAL_DEVICE_TYPE_DISCRETE_GPU"
        && device_type != "PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU"
    {
        return;
    }
    let name = record
        .name
        .clone()
        .unwrap_or_else(|| "unnamed-vulkan-device".to_owned());
    devices.push(PhysicalDevice { name });
}

fn sha256_hex(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn default_distro_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {command} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn shell_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

fn bool_as_int(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_package_lines, parse_physical_devices, shell_quote};

    #[test]
    fn strips_comments_and_blanks() {
        let parsed = parse_package_lines(
            "\n  foo  \nbar # keep\n# comment only\nbaz#inline\n\n",
        );
        assert_eq!(parsed, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn shell_quotes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
    }

    #[test]
    fn keeps_only_physical_vulkan_devices() {
        let devices = parse_physical_devices(
            "GPU0:\n    deviceType = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU\n    deviceName = AMD Radeon Graphics\nGPU1:\n    deviceType = PHYSICAL_DEVICE_TYPE_CPU\n    deviceName = llvmpipe\n",
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "AMD Radeon Graphics");
    }
}
