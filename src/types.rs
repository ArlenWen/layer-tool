use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Container metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetadata {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub image_sha256: String,
    pub created: DateTime<Utc>,
    pub state: String,
    pub status: String,
    pub labels: HashMap<String, String>,
    pub mounts: Vec<MountInfo>,
}

/// Mount information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub rw: bool,
    pub propagation: String,
}

/// Docker daemon information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerInfo {
    pub id: String,
    pub containers: u32,
    pub containers_running: u32,
    pub containers_paused: u32,
    pub containers_stopped: u32,
    pub images: u32,
    pub driver: String,
    pub driver_status: Vec<(String, String)>,
    pub system_status: Option<Vec<(String, String)>>,
    pub plugins: PluginInfo,
    pub memory_limit: bool,
    pub swap_limit: bool,
    pub kernel_memory: bool,
    pub cpu_cfs_period: bool,
    pub cpu_cfs_quota: bool,
    pub cpu_shares: bool,
    pub cpu_set: bool,
    pub pids_limit: bool,
    pub ipv4_forwarding: bool,
    pub bridge_nf_iptables: bool,
    pub bridge_nf_ip6tables: bool,
    pub debug: bool,
    pub nfd: u32,
    pub oom_kill_disable: bool,
    pub n_goroutines: u32,
    pub system_time: DateTime<Utc>,
    pub logging_driver: String,
    pub cgroup_driver: String,
    pub n_events_listener: u32,
    pub kernel_version: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    pub index_server_address: String,
    pub registry_config: RegistryConfig,
    pub ncpu: u32,
    pub mem_total: u64,
    pub generic_resources: Option<Vec<GenericResource>>,
    pub docker_root_dir: String,
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
    pub name: String,
    pub labels: Vec<String>,
    pub experimental_build: bool,
    pub server_version: String,
    pub cluster_store: String,
    pub cluster_advertise: String,
    pub runtimes: HashMap<String, Runtime>,
    pub default_runtime: String,
    pub swarm: SwarmInfo,
    pub live_restore_enabled: bool,
    pub isolation: String,
    pub init_binary: String,
    pub containerd_commit: CommitInfo,
    pub runc_commit: CommitInfo,
    pub init_commit: CommitInfo,
    pub security_options: Vec<String>,
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub volume: Vec<String>,
    pub network: Vec<String>,
    pub authorization: Option<Vec<String>>,
    pub log: Vec<String>,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub allow_nondistributable_artifacts_cidrs: Option<Vec<String>>,
    pub allow_nondistributable_artifacts_hostnames: Option<Vec<String>>,
    pub insecure_registry_cidrs: Option<Vec<String>>,
    pub index_configs: HashMap<String, IndexConfig>,
    pub mirrors: Vec<String>,
}

/// Index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    pub name: String,
    pub mirrors: Vec<String>,
    pub secure: bool,
    pub official: bool,
}

/// Generic resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResource {
    pub named_resource_spec: Option<NamedResourceSpec>,
    pub discrete_resource_spec: Option<DiscreteResourceSpec>,
}

/// Named resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedResourceSpec {
    pub kind: String,
    pub value: String,
}

/// Discrete resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteResourceSpec {
    pub kind: String,
    pub value: i64,
}

/// Runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub path: String,
    pub runtime_args: Option<Vec<String>>,
}

/// Swarm information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInfo {
    pub node_id: String,
    pub node_addr: String,
    pub local_node_state: String,
    pub control_available: bool,
    pub error: String,
    pub remote_managers: Option<Vec<PeerNode>>,
    pub nodes: Option<u32>,
    pub managers: Option<u32>,
    pub cluster: Option<ClusterInfo>,
}

/// Peer node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerNode {
    pub node_id: String,
    pub addr: String,
}

/// Cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub id: String,
    pub version: ObjectVersion,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub spec: ClusterSpec,
}

/// Object version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectVersion {
    pub index: u64,
}

/// Cluster specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSpec {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub orchestration: OrchestrationConfig,
    pub raft: RaftConfig,
    pub dispatcher: DispatcherConfig,
    pub ca_config: CAConfig,
    pub encryption_config: EncryptionConfig,
    pub task_defaults: TaskDefaults,
}

/// Orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub task_history_retention_limit: Option<i64>,
}

/// Raft configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    pub snapshot_interval: Option<u64>,
    pub keep_old_snapshots: Option<u64>,
    pub log_entries_for_slow_followers: Option<u64>,
    pub election_tick: Option<i32>,
    pub heartbeat_tick: Option<i32>,
}

/// Dispatcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    pub heartbeat_period: Option<u64>,
}

/// CA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAConfig {
    pub node_cert_expiry: Option<u64>,
    pub external_cas: Option<Vec<ExternalCA>>,
    pub signing_ca_cert: Option<String>,
    pub signing_ca_key: Option<String>,
    pub force_rotate: Option<u64>,
}

/// External CA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCA {
    pub protocol: String,
    pub url: String,
    pub options: Option<HashMap<String, String>>,
    pub ca_cert: Option<String>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub auto_lock_managers: bool,
}

/// Task defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefaults {
    pub log_driver: Option<LogDriver>,
}

/// Log driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDriver {
    pub name: String,
    pub options: Option<HashMap<String, String>>,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub expected: String,
}

/// Export data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub created: DateTime<Utc>,
    pub container_metadata: ContainerMetadata,
    pub docker_info: DockerInfo,
    pub layer_checksum: String,
    pub compressed: bool,
}

/// Check options
#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub skip_image: bool,
    pub skip_storage: bool,
    pub skip_os: bool,
    pub skip_arch: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            skip_image: false,
            skip_storage: false,
            skip_os: false,
            skip_arch: false,
        }
    }
}
