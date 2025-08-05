use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::types::{ContainerMetadata, DockerInfo};

/// Docker client for interacting with Docker daemon
pub struct DockerClient;

impl DockerClient {
    pub fn new() -> Self {
        Self
    }

    /// Get container metadata by container ID
    pub fn get_container_metadata(&self, container_id: &str) -> Result<ContainerMetadata> {
        let output = Command::new("docker")
            .args(&["inspect", container_id])
            .output()
            .context("Failed to execute docker inspect command")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Docker inspect failed: {}", error));
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse docker inspect output as UTF-8")?;

        let inspect_data: Vec<Value> = serde_json::from_str(&stdout)
            .context("Failed to parse docker inspect JSON output")?;

        if inspect_data.is_empty() {
            return Err(anyhow!("Container not found: {}", container_id));
        }

        let container = &inspect_data[0];
        self.parse_container_metadata(container)
    }

    /// Get Docker daemon information
    pub fn get_docker_info(&self) -> Result<DockerInfo> {
        let output = Command::new("docker")
            .args(&["info", "--format", "{{json .}}"])
            .output()
            .context("Failed to execute docker info command")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Docker info failed: {}", error));
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse docker info output as UTF-8")?;

        let info_data: Value = serde_json::from_str(&stdout)
            .context("Failed to parse docker info JSON output")?;

        self.parse_docker_info(&info_data)
    }

    /// Get the path to container's layer directory
    pub fn get_container_layer_path(&self, container_id: &str) -> Result<PathBuf> {
        let _metadata = self.get_container_metadata(container_id)?;

        // Try to get the layer path from container metadata
        let output = Command::new("docker")
            .args(&["inspect", "--format", "{{.GraphDriver.Data.MergedDir}}", container_id])
            .output()
            .context("Failed to get container layer path")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get container layer path: {}", error));
        }

        let merged_dir = String::from_utf8(output.stdout)
            .context("Failed to parse layer path as UTF-8")?
            .trim()
            .to_string();

        if merged_dir.is_empty() {
            return Err(anyhow!("Container layer path is empty"));
        }

        // Get the parent directory which contains upper, lower, work, merged
        let layer_path = Path::new(&merged_dir)
            .parent()
            .ok_or_else(|| anyhow!("Invalid layer path: {}", merged_dir))?;

        Ok(layer_path.to_path_buf())
    }

    /// Get the upper layer directory path (read-write layer) with enhanced resolution
    /// Returns the path directly without checking if the directory exists
    pub fn get_upper_layer_path(&self, container_id: &str) -> Result<PathBuf> {
        // Method 1: Try to get UpperDir directly from GraphDriver.Data
        if let Ok(upper_path) = self.get_upper_layer_path_direct(container_id) {
            println!("Found upper layer using direct method: {:?}", upper_path);
            return Ok(upper_path);
        }

        // Method 2: Try the traditional approach (MergedDir parent + upper)
        if let Ok(upper_path) = self.get_upper_layer_path_traditional(container_id) {
            println!("Found upper layer using traditional method: {:?}", upper_path);
            return Ok(upper_path);
        }

        // Method 3: Try to find the upper layer by inspecting the overlay2 structure
        if let Ok(upper_path) = self.get_upper_layer_path_by_inspection(container_id) {
            println!("Found upper layer using inspection method: {:?}", upper_path);
            return Ok(upper_path);
        }

        // If all methods fail, provide detailed error information
        self.provide_detailed_layer_error(container_id)
    }

    /// Method 1: Try to get UpperDir directly from GraphDriver.Data
    fn get_upper_layer_path_direct(&self, container_id: &str) -> Result<PathBuf> {
        let output = Command::new("docker")
            .args(&["inspect", "--format", "{{.GraphDriver.Data.UpperDir}}", container_id])
            .output()
            .context("Failed to get container upper layer path directly")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get container upper layer path directly: {}", error));
        }

        let upper_dir = String::from_utf8(output.stdout)
            .context("Failed to parse upper layer path as UTF-8")?
            .trim()
            .to_string();

        if upper_dir.is_empty() || upper_dir == "<no value>" {
            return Err(anyhow!("Container upper layer path is empty or not available"));
        }

        let upper_path = PathBuf::from(upper_dir);

        // Use the returned path directly, regardless of whether it's "upper" or "diff"
        println!("Using container layer directory: {:?}", upper_path);

        Ok(upper_path)
    }

    /// Method 2: Traditional approach using MergedDir parent + upper
    fn get_upper_layer_path_traditional(&self, container_id: &str) -> Result<PathBuf> {
        let layer_path = self.get_container_layer_path(container_id)?;
        Ok(layer_path.join("upper"))
    }

    /// Method 3: Inspect overlay2 structure to find the upper layer
    fn get_upper_layer_path_by_inspection(&self, container_id: &str) -> Result<PathBuf> {
        // Get full GraphDriver data
        let output = Command::new("docker")
            .args(&["inspect", "--format", "{{json .GraphDriver}}", container_id])
            .output()
            .context("Failed to get container GraphDriver data")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get container GraphDriver data: {}", error));
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse GraphDriver data as UTF-8")?;

        let graph_driver: Value = serde_json::from_str(&stdout)
            .context("Failed to parse GraphDriver JSON data")?;

        // Try to extract the layer ID from various possible locations
        if let Some(data) = graph_driver["Data"].as_object() {
            // Look for any path that might contain the layer ID
            for (key, value) in data {
                if let Some(path_str) = value.as_str() {
                    if path_str.contains("/overlay2/") && (key.contains("Dir") || key.contains("Path")) {
                        // Extract the layer ID from the path
                        if let Some(layer_id) = self.extract_layer_id_from_path(path_str) {
                            let upper_path = PathBuf::from(format!("/var/lib/docker/overlay2/{}/upper", layer_id));
                            if upper_path.exists() {
                                return Ok(upper_path);
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!("Could not determine upper layer path from GraphDriver inspection"))
    }

    /// Extract layer ID from overlay2 path
    fn extract_layer_id_from_path(&self, path: &str) -> Option<String> {
        // Look for pattern like /var/lib/docker/overlay2/{layer_id}/...
        if let Some(overlay2_pos) = path.find("/overlay2/") {
            let after_overlay2 = &path[overlay2_pos + 10..]; // Skip "/overlay2/"
            if let Some(slash_pos) = after_overlay2.find('/') {
                return Some(after_overlay2[..slash_pos].to_string());
            } else {
                return Some(after_overlay2.to_string());
            }
        }
        None
    }

    /// Provide detailed error information when upper layer path cannot be found
    fn provide_detailed_layer_error(&self, container_id: &str) -> Result<PathBuf> {
        println!("=== DEBUGGING CONTAINER LAYER PATHS ===");

        // Get full container inspect data for debugging
        let output = Command::new("docker")
            .args(&["inspect", container_id])
            .output()
            .context("Failed to get container inspect data for debugging")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(inspect_data) = serde_json::from_str::<Vec<Value>>(&stdout) {
                if let Some(container) = inspect_data.first() {
                    if let Some(graph_driver) = container.get("GraphDriver") {
                        println!("GraphDriver data: {}", serde_json::to_string_pretty(graph_driver).unwrap_or_default());

                        if let Some(data) = graph_driver.get("Data") {
                            if let Some(data_obj) = data.as_object() {
                                for (key, value) in data_obj {
                                    println!("  {}: {}", key, value.as_str().unwrap_or("N/A"));

                                    // Check if any of these paths exist
                                    if let Some(path_str) = value.as_str() {
                                        let path = PathBuf::from(path_str);
                                        println!("    Path exists: {}", path.exists());
                                        if path.exists() && path.is_dir() {
                                            if let Ok(entries) = std::fs::read_dir(&path) {
                                                let count = entries.count();
                                                println!("    Directory contains {} entries", count);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Check container state
                    if let Some(state) = container.get("State") {
                        println!("Container State: {}", serde_json::to_string_pretty(state).unwrap_or_default());
                    }
                }
            }
        }

        // Check if Docker daemon is using overlay2
        let info_output = Command::new("docker")
            .args(&["info", "--format", "{{.Driver}}"])
            .output();

        if let Ok(output) = info_output {
            if output.status.success() {
                let driver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("Docker storage driver: {}", driver);
                if driver != "overlay2" {
                    println!("WARNING: This tool is designed for overlay2 storage driver, but Docker is using: {}", driver);
                }
            }
        }

        // List overlay2 directory to see what's available
        let overlay2_dir = PathBuf::from("/var/lib/docker/overlay2");
        if overlay2_dir.exists() {
            println!("Overlay2 directory exists: {:?}", overlay2_dir);
            if let Ok(entries) = std::fs::read_dir(&overlay2_dir) {
                let mut count = 0;
                for entry in entries {
                    if let Ok(entry) = entry {
                        count += 1;
                        if count <= 5 { // Show first 5 entries
                            println!("  Found layer: {:?}", entry.file_name());
                        }
                    }
                }
                println!("  Total overlay2 layers found: {}", count);
            }
        } else {
            println!("Overlay2 directory does not exist: {:?}", overlay2_dir);
        }

        Err(anyhow!(
            "Container upper layer directory not found after trying all methods. \
            Container ID: {}. Please check the debugging information above.",
            container_id
        ))
    }

    /// Check if container exists
    pub fn container_exists(&self, container_id: &str) -> Result<bool> {
        let output = Command::new("docker")
            .args(&["inspect", container_id])
            .output()
            .context("Failed to check if container exists")?;

        Ok(output.status.success())
    }

    /// Validate container state and readiness for layer operations
    pub fn validate_container_for_layer_operations(&self, container_id: &str) -> Result<()> {
        // Check if container exists
        if !self.container_exists(container_id)? {
            return Err(anyhow!("Container not found: {}", container_id));
        }

        // Get container metadata to check state
        let metadata = self.get_container_metadata(container_id)?;

        // Check if container is in a valid state for layer operations
        let state_lower = metadata.state.to_lowercase();
        if state_lower == "removing" || state_lower == "dead" {
            return Err(anyhow!(
                "Container is in '{}' state and cannot be used for layer operations",
                metadata.state
            ));
        }

        // Check storage driver compatibility
        let docker_info = self.get_docker_info()?;
        if docker_info.driver != "overlay2" {
            println!("WARNING: This tool is optimized for overlay2 storage driver, but Docker is using: {}", docker_info.driver);
            println!("Layer operations may not work correctly with other storage drivers.");
        }

        println!("Container validation passed:");
        println!("  Container ID: {}", metadata.id);
        println!("  Container Name: {}", metadata.name);
        println!("  State: {}", metadata.state);
        println!("  Storage Driver: {}", docker_info.driver);

        Ok(())
    }

    /// Parse container metadata from Docker inspect JSON
    fn parse_container_metadata(&self, container: &Value) -> Result<ContainerMetadata> {
        use chrono::{DateTime, Utc};
        use std::collections::HashMap;
        use crate::types::MountInfo;

        let id = container["Id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing container ID"))?
            .to_string();

        let name = container["Name"]
            .as_str()
            .unwrap_or("")
            .trim_start_matches('/')
            .to_string();

        let config = &container["Config"];
        let image = config["Image"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing container image"))?
            .to_string();

        let image_id = container["Image"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing container image ID"))?
            .to_string();

        // Extract SHA256 from image ID
        let image_sha256 = if image_id.starts_with("sha256:") {
            image_id.clone()
        } else {
            format!("sha256:{}", image_id)
        };

        let created_str = container["Created"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing container created timestamp"))?;
        let created = DateTime::parse_from_rfc3339(created_str)
            .context("Failed to parse created timestamp")?
            .with_timezone(&Utc);

        let state = &container["State"];
        let state_status = state["Status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let status = format!(
            "{} {}",
            state_status,
            state["StartedAt"].as_str().unwrap_or("")
        );

        // Parse labels
        let mut labels = HashMap::new();
        if let Some(labels_obj) = config["Labels"].as_object() {
            for (key, value) in labels_obj {
                if let Some(value_str) = value.as_str() {
                    labels.insert(key.clone(), value_str.to_string());
                }
            }
        }

        // Parse mounts
        let mut mounts = Vec::new();
        if let Some(mounts_array) = container["Mounts"].as_array() {
            for mount in mounts_array {
                if let (Some(source), Some(destination), Some(mode)) = (
                    mount["Source"].as_str(),
                    mount["Destination"].as_str(),
                    mount["Mode"].as_str(),
                ) {
                    mounts.push(MountInfo {
                        source: source.to_string(),
                        destination: destination.to_string(),
                        mode: mode.to_string(),
                        rw: mount["RW"].as_bool().unwrap_or(false),
                        propagation: mount["Propagation"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
        }

        Ok(ContainerMetadata {
            id,
            name,
            image,
            image_id,
            image_sha256,
            created,
            state: state_status,
            status,
            labels,
            mounts,
        })
    }

    /// Parse Docker info from JSON (simplified version)
    fn parse_docker_info(&self, info: &Value) -> Result<DockerInfo> {
        use chrono::Utc;
        use std::collections::HashMap;
        use crate::types::*;

        // This is a simplified parser - in a real implementation,
        // you'd want to parse all fields properly
        let docker_info = DockerInfo {
            id: info["ID"].as_str().unwrap_or("").to_string(),
            containers: info["Containers"].as_u64().unwrap_or(0) as u32,
            containers_running: info["ContainersRunning"].as_u64().unwrap_or(0) as u32,
            containers_paused: info["ContainersPaused"].as_u64().unwrap_or(0) as u32,
            containers_stopped: info["ContainersStopped"].as_u64().unwrap_or(0) as u32,
            images: info["Images"].as_u64().unwrap_or(0) as u32,
            driver: info["Driver"].as_str().unwrap_or("").to_string(),
            driver_status: Vec::new(), // Simplified
            system_status: None,
            plugins: PluginInfo {
                volume: Vec::new(),
                network: Vec::new(),
                authorization: None,
                log: Vec::new(),
            },
            memory_limit: info["MemoryLimit"].as_bool().unwrap_or(false),
            swap_limit: info["SwapLimit"].as_bool().unwrap_or(false),
            kernel_memory: info["KernelMemory"].as_bool().unwrap_or(false),
            cpu_cfs_period: info["CpuCfsPeriod"].as_bool().unwrap_or(false),
            cpu_cfs_quota: info["CpuCfsQuota"].as_bool().unwrap_or(false),
            cpu_shares: info["CPUShares"].as_bool().unwrap_or(false),
            cpu_set: info["CPUSet"].as_bool().unwrap_or(false),
            pids_limit: info["PidsLimit"].as_bool().unwrap_or(false),
            ipv4_forwarding: info["IPv4Forwarding"].as_bool().unwrap_or(false),
            bridge_nf_iptables: info["BridgeNfIptables"].as_bool().unwrap_or(false),
            bridge_nf_ip6tables: info["BridgeNfIp6tables"].as_bool().unwrap_or(false),
            debug: info["Debug"].as_bool().unwrap_or(false),
            nfd: info["NFd"].as_u64().unwrap_or(0) as u32,
            oom_kill_disable: info["OomKillDisable"].as_bool().unwrap_or(false),
            n_goroutines: info["NGoroutines"].as_u64().unwrap_or(0) as u32,
            system_time: Utc::now(), // Simplified
            logging_driver: info["LoggingDriver"].as_str().unwrap_or("").to_string(),
            cgroup_driver: info["CgroupDriver"].as_str().unwrap_or("").to_string(),
            n_events_listener: info["NEventsListener"].as_u64().unwrap_or(0) as u32,
            kernel_version: info["KernelVersion"].as_str().unwrap_or("").to_string(),
            operating_system: info["OperatingSystem"].as_str().unwrap_or("").to_string(),
            os_type: info["OSType"].as_str().unwrap_or("").to_string(),
            architecture: info["Architecture"].as_str().unwrap_or("").to_string(),
            index_server_address: info["IndexServerAddress"].as_str().unwrap_or("").to_string(),
            registry_config: RegistryConfig {
                allow_nondistributable_artifacts_cidrs: None,
                allow_nondistributable_artifacts_hostnames: None,
                insecure_registry_cidrs: None,
                index_configs: HashMap::new(),
                mirrors: Vec::new(),
            },
            ncpu: info["NCPU"].as_u64().unwrap_or(0) as u32,
            mem_total: info["MemTotal"].as_u64().unwrap_or(0),
            generic_resources: None,
            docker_root_dir: info["DockerRootDir"].as_str().unwrap_or("").to_string(),
            http_proxy: info["HttpProxy"].as_str().unwrap_or("").to_string(),
            https_proxy: info["HttpsProxy"].as_str().unwrap_or("").to_string(),
            no_proxy: info["NoProxy"].as_str().unwrap_or("").to_string(),
            name: info["Name"].as_str().unwrap_or("").to_string(),
            labels: Vec::new(),
            experimental_build: info["ExperimentalBuild"].as_bool().unwrap_or(false),
            server_version: info["ServerVersion"].as_str().unwrap_or("").to_string(),
            cluster_store: info["ClusterStore"].as_str().unwrap_or("").to_string(),
            cluster_advertise: info["ClusterAdvertise"].as_str().unwrap_or("").to_string(),
            runtimes: HashMap::new(),
            default_runtime: info["DefaultRuntime"].as_str().unwrap_or("").to_string(),
            swarm: SwarmInfo {
                node_id: "".to_string(),
                node_addr: "".to_string(),
                local_node_state: "".to_string(),
                control_available: false,
                error: "".to_string(),
                remote_managers: None,
                nodes: None,
                managers: None,
                cluster: None,
            },
            live_restore_enabled: info["LiveRestoreEnabled"].as_bool().unwrap_or(false),
            isolation: info["Isolation"].as_str().unwrap_or("").to_string(),
            init_binary: info["InitBinary"].as_str().unwrap_or("").to_string(),
            containerd_commit: CommitInfo {
                id: "".to_string(),
                expected: "".to_string(),
            },
            runc_commit: CommitInfo {
                id: "".to_string(),
                expected: "".to_string(),
            },
            init_commit: CommitInfo {
                id: "".to_string(),
                expected: "".to_string(),
            },
            security_options: Vec::new(),
        };

        Ok(docker_info)
    }
}
