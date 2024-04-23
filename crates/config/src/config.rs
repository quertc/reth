//! Configuration files.

use reth_discv4::Discv4Config;
use reth_network::{NetworkConfigBuilder, PeersConfig, SessionsConfig};
use reth_primitives::PruneModes;
use secp256k1::SecretKey;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// Configuration for the reth node.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Config {
    /// Configuration for each stage in the pipeline.
    // TODO(onbjerg): Can we make this easier to maintain when we add/remove stages?
    pub stages: StageConfig,
    /// Configuration for pruning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prune: Option<PruneConfig>,
    /// Configuration for the discovery service.
    pub peers: PeersConfig,
    /// Configuration for peer sessions.
    pub sessions: SessionsConfig,
    /// Configuration for the EVM bytecode compiler.
    pub compiler: CompilerConfig,
}

impl Config {
    /// Initializes network config from read data
    pub fn network_config(
        &self,
        nat_resolution_method: reth_net_nat::NatResolver,
        peers_file: Option<PathBuf>,
        secret_key: SecretKey,
    ) -> NetworkConfigBuilder {
        let peer_config = self
            .peers
            .clone()
            .with_basic_nodes_from_file(peers_file)
            .unwrap_or_else(|_| self.peers.clone());

        let discv4 =
            Discv4Config::builder().external_ip_resolver(Some(nat_resolution_method)).clone();
        NetworkConfigBuilder::new(secret_key)
            .sessions_config(self.sessions.clone())
            .peer_config(peer_config)
            .discovery(discv4)
    }
}

/// Configuration for each stage in the pipeline.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct StageConfig {
    /// Header stage configuration.
    pub headers: HeadersConfig,
    /// Body stage configuration.
    pub bodies: BodiesConfig,
    /// Sender Recovery stage configuration.
    pub sender_recovery: SenderRecoveryConfig,
    /// Execution stage configuration.
    pub execution: ExecutionConfig,
    /// Account Hashing stage configuration.
    pub account_hashing: HashingConfig,
    /// Storage Hashing stage configuration.
    pub storage_hashing: HashingConfig,
    /// Merkle stage configuration.
    pub merkle: MerkleConfig,
    /// Transaction Lookup stage configuration.
    pub transaction_lookup: TransactionLookupConfig,
    /// Index Account History stage configuration.
    pub index_account_history: IndexHistoryConfig,
    /// Index Storage History stage configuration.
    pub index_storage_history: IndexHistoryConfig,
    /// Common ETL related configuration.
    pub etl: EtlConfig,
}

/// Header stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct HeadersConfig {
    /// The maximum number of requests to send concurrently.
    ///
    /// Default: 100
    pub downloader_max_concurrent_requests: usize,
    /// The minimum number of requests to send concurrently.
    ///
    /// Default: 5
    pub downloader_min_concurrent_requests: usize,
    /// Maximum amount of responses to buffer internally.
    /// The response contains multiple headers.
    pub downloader_max_buffered_responses: usize,
    /// The maximum number of headers to request from a peer at a time.
    pub downloader_request_limit: u64,
    /// The maximum number of headers to download before committing progress to the database.
    pub commit_threshold: u64,
}

impl Default for HeadersConfig {
    fn default() -> Self {
        Self {
            commit_threshold: 10_000,
            downloader_request_limit: 1_000,
            downloader_max_concurrent_requests: 100,
            downloader_min_concurrent_requests: 5,
            downloader_max_buffered_responses: 100,
        }
    }
}

/// Body stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct BodiesConfig {
    /// The batch size of non-empty blocks per one request
    ///
    /// Default: 200
    pub downloader_request_limit: u64,
    /// The maximum number of block bodies returned at once from the stream
    ///
    /// Default: 1_000
    pub downloader_stream_batch_size: usize,
    /// The size of the internal block buffer in bytes.
    ///
    /// Default: 2GB
    pub downloader_max_buffered_blocks_size_bytes: usize,
    /// The minimum number of requests to send concurrently.
    ///
    /// Default: 5
    pub downloader_min_concurrent_requests: usize,
    /// The maximum number of requests to send concurrently.
    /// This is equal to the max number of peers.
    ///
    /// Default: 100
    pub downloader_max_concurrent_requests: usize,
}

impl Default for BodiesConfig {
    fn default() -> Self {
        Self {
            downloader_request_limit: 200,
            downloader_stream_batch_size: 1_000,
            downloader_max_buffered_blocks_size_bytes: 2 * 1024 * 1024 * 1024, // ~2GB
            downloader_min_concurrent_requests: 5,
            downloader_max_concurrent_requests: 100,
        }
    }
}

/// Sender recovery stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct SenderRecoveryConfig {
    /// The maximum number of transactions to process before committing progress to the database.
    pub commit_threshold: u64,
}

impl Default for SenderRecoveryConfig {
    fn default() -> Self {
        Self { commit_threshold: 5_000_000 }
    }
}

/// Execution stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// The maximum number of blocks to process before the execution stage commits.
    pub max_blocks: Option<u64>,
    /// The maximum number of state changes to keep in memory before the execution stage commits.
    pub max_changes: Option<u64>,
    /// The maximum cumulative amount of gas to process before the execution stage commits.
    pub max_cumulative_gas: Option<u64>,
    /// The maximum time spent on blocks processing before the execution stage commits.
    #[serde(
        serialize_with = "humantime_serde::serialize",
        deserialize_with = "deserialize_duration"
    )]
    pub max_duration: Option<Duration>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_blocks: Some(500_000),
            max_changes: Some(5_000_000),
            // 50k full blocks of 30M gas
            max_cumulative_gas: Some(30_000_000 * 50_000),
            // 10 minutes
            max_duration: Some(Duration::from_secs(10 * 60)),
        }
    }
}

/// Hashing stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct HashingConfig {
    /// The threshold (in number of blocks) for switching between
    /// incremental hashing and full hashing.
    pub clean_threshold: u64,
    /// The maximum number of entities to process before committing progress to the database.
    pub commit_threshold: u64,
}

impl Default for HashingConfig {
    fn default() -> Self {
        Self { clean_threshold: 500_000, commit_threshold: 100_000 }
    }
}

/// Merkle stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct MerkleConfig {
    /// The threshold (in number of blocks) for switching from incremental trie building of changes
    /// to whole rebuild.
    pub clean_threshold: u64,
}

impl Default for MerkleConfig {
    fn default() -> Self {
        Self { clean_threshold: 5_000 }
    }
}

/// Transaction Lookup stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct TransactionLookupConfig {
    /// The maximum number of transactions to process before writing to disk.
    pub chunk_size: u64,
}

impl Default for TransactionLookupConfig {
    fn default() -> Self {
        Self { chunk_size: 5_000_000 }
    }
}

/// Common ETL related configuration.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct EtlConfig {
    /// Data directory where temporary files are created.
    pub dir: Option<PathBuf>,
    /// The maximum size in bytes of data held in memory before being flushed to disk as a file.
    pub file_size: usize,
}

impl Default for EtlConfig {
    fn default() -> Self {
        Self { dir: None, file_size: Self::default_file_size() }
    }
}

impl EtlConfig {
    /// Creates an ETL configuration
    pub fn new(dir: Option<PathBuf>, file_size: usize) -> Self {
        Self { dir, file_size }
    }

    /// Return default ETL directory from datadir path.
    pub fn from_datadir(path: &Path) -> PathBuf {
        path.join("etl-tmp")
    }

    /// Default size in bytes of data held in memory before being flushed to disk as a file.
    pub const fn default_file_size() -> usize {
        // 500 MB
        500 * (1024 * 1024)
    }
}

/// History stage configuration.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct IndexHistoryConfig {
    /// The maximum number of blocks to process before committing progress to the database.
    pub commit_threshold: u64,
}

impl Default for IndexHistoryConfig {
    fn default() -> Self {
        Self { commit_threshold: 100_000 }
    }
}

/// Pruning configuration.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct PruneConfig {
    /// Minimum pruning interval measured in blocks.
    pub block_interval: usize,
    /// Pruning configuration for every part of the data that can be pruned.
    #[serde(alias = "parts")]
    pub segments: PruneModes,
}

impl Default for PruneConfig {
    fn default() -> Self {
        Self { block_interval: 5, segments: PruneModes::none() }
    }
}

/// EVM bytecode compiler configuration.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CompilerConfig {
    /// The block number at which the compiled contracts stop being run.
    ///
    /// Defaults to 19 million.
    pub end_block: u64,
    /// Path to a file that contains all the contracts to compile.
    ///
    /// Defaults to `<datadir>/contracts.toml`.
    pub contracts_file: Option<PathBuf>,
    /// Directory in which intermediate artifacts, metadata, and results are stored.
    ///
    /// Defaults to `<datadir>/compiled_contracts`.
    pub out_dir: Option<PathBuf>,
    /// Which C compiler to use for linking.
    pub cc: Option<PathBuf>,
    /// Additional arguments to pass to the C compiler when linking.
    pub cflags: Vec<String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            end_block: 19_000_000,
            contracts_file: None,
            out_dir: None,
            cc: None,
            cflags: vec![],
        }
    }
}

/// Helper type to support older versions of Duration deserialization.
fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyDuration {
        #[serde(deserialize_with = "humantime_serde::deserialize")]
        Human(Option<Duration>),
        Duration(Option<Duration>),
    }

    AnyDuration::deserialize(deserializer).map(|d| match d {
        AnyDuration::Human(duration) | AnyDuration::Duration(duration) => duration,
    })
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::time::Duration;

    const EXTENSION: &str = "toml";

    fn with_tempdir(filename: &str, proc: fn(&std::path::Path)) {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(filename).with_extension(EXTENSION);

        proc(&config_path);

        temp_dir.close().unwrap()
    }

    #[test]
    fn test_store_config() {
        with_tempdir("config-store-test", |config_path| {
            let config = Config::default();
            confy::store_path(config_path, config).expect("Failed to store config");
        })
    }

    #[test]
    fn test_load_config() {
        with_tempdir("config-load-test", |config_path| {
            let config = Config::default();
            confy::store_path(config_path, &config).unwrap();

            let loaded_config: Config = confy::load_path(config_path).unwrap();
            assert_eq!(config, loaded_config);
        })
    }

    #[test]
    fn test_load_execution_stage() {
        with_tempdir("config-load-test", |config_path| {
            let mut config = Config::default();
            config.stages.execution.max_duration = Some(Duration::from_secs(10 * 60));
            confy::store_path(config_path, &config).unwrap();

            let loaded_config: Config = confy::load_path(config_path).unwrap();
            assert_eq!(config, loaded_config);
        })
    }

    // ensures config deserialization is backwards compatible
    #[test]
    fn test_backwards_compatibility() {
        let alpha_0_0_8 = r"#
[stages.headers]
downloader_max_concurrent_requests = 100
downloader_min_concurrent_requests = 5
downloader_max_buffered_responses = 100
downloader_request_limit = 1000
commit_threshold = 10000

[stages.bodies]
downloader_request_limit = 200
downloader_stream_batch_size = 1000
downloader_max_buffered_blocks_size_bytes = 2147483648
downloader_min_concurrent_requests = 5
downloader_max_concurrent_requests = 100

[stages.sender_recovery]
commit_threshold = 5000000

[stages.execution]
max_blocks = 500000
max_changes = 5000000

[stages.account_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.storage_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.merkle]
clean_threshold = 50000

[stages.transaction_lookup]
chunk_size = 5000000

[stages.index_account_history]
commit_threshold = 100000

[stages.index_storage_history]
commit_threshold = 100000

[peers]
refill_slots_interval = '1s'
trusted_nodes = []
connect_trusted_nodes_only = false
max_backoff_count = 5
ban_duration = '12h'

[peers.connection_info]
max_outbound = 100
max_inbound = 30

[peers.reputation_weights]
bad_message = -16384
bad_block = -16384
bad_transactions = -16384
already_seen_transactions = 0
timeout = -4096
bad_protocol = -2147483648
failed_to_connect = -25600
dropped = -4096

[peers.backoff_durations]
low = '30s'
medium = '3m'
high = '15m'
max = '1h'

[sessions]
session_command_buffer = 32
session_event_buffer = 260

[sessions.limits]

[sessions.initial_internal_request_timeout]
secs = 20
nanos = 0

[sessions.protocol_breach_request_timeout]
secs = 120
nanos = 0

[prune]
block_interval = 5

[prune.parts]
sender_recovery = { distance = 16384 }
transaction_lookup = 'full'
receipts = { before = 1920000 }
account_history = { distance = 16384 }
storage_history = { distance = 16384 }
[prune.parts.receipts_log_filter]
'0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48' = { before = 17000000 }
'0xdac17f958d2ee523a2206206994597c13d831ec7' = { distance = 1000 }
#";
        let _conf: Config = toml::from_str(alpha_0_0_8).unwrap();

        let alpha_0_0_11 = r"#
[prune.segments]
sender_recovery = { distance = 16384 }
transaction_lookup = 'full'
receipts = { before = 1920000 }
account_history = { distance = 16384 }
storage_history = { distance = 16384 }
[prune.segments.receipts_log_filter]
'0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48' = { before = 17000000 }
'0xdac17f958d2ee523a2206206994597c13d831ec7' = { distance = 1000 }
#";
        let _conf: Config = toml::from_str(alpha_0_0_11).unwrap();

        let alpha_0_0_18 = r"#
[stages.headers]
downloader_max_concurrent_requests = 100
downloader_min_concurrent_requests = 5
downloader_max_buffered_responses = 100
downloader_request_limit = 1000
commit_threshold = 10000

[stages.total_difficulty]
commit_threshold = 100000

[stages.bodies]
downloader_request_limit = 200
downloader_stream_batch_size = 1000
downloader_max_buffered_blocks_size_bytes = 2147483648
downloader_min_concurrent_requests = 5
downloader_max_concurrent_requests = 100

[stages.sender_recovery]
commit_threshold = 5000000

[stages.execution]
max_blocks = 500000
max_changes = 5000000
max_cumulative_gas = 1500000000000
[stages.execution.max_duration]
secs = 600
nanos = 0

[stages.account_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.storage_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.merkle]
clean_threshold = 50000

[stages.transaction_lookup]
commit_threshold = 5000000

[stages.index_account_history]
commit_threshold = 100000

[stages.index_storage_history]
commit_threshold = 100000

[peers]
refill_slots_interval = '5s'
trusted_nodes = []
connect_trusted_nodes_only = false
max_backoff_count = 5
ban_duration = '12h'

[peers.connection_info]
max_outbound = 100
max_inbound = 30
max_concurrent_outbound_dials = 10

[peers.reputation_weights]
bad_message = -16384
bad_block = -16384
bad_transactions = -16384
already_seen_transactions = 0
timeout = -4096
bad_protocol = -2147483648
failed_to_connect = -25600
dropped = -4096
bad_announcement = -1024

[peers.backoff_durations]
low = '30s'
medium = '3m'
high = '15m'
max = '1h'

[sessions]
session_command_buffer = 32
session_event_buffer = 260

[sessions.limits]

[sessions.initial_internal_request_timeout]
secs = 20
nanos = 0

[sessions.protocol_breach_request_timeout]
secs = 120
nanos = 0
#";
        let conf: Config = toml::from_str(alpha_0_0_18).unwrap();
        assert_eq!(conf.stages.execution.max_duration, Some(Duration::from_secs(10 * 60)));

        let alpha_0_0_19 = r"#
[stages.headers]
downloader_max_concurrent_requests = 100
downloader_min_concurrent_requests = 5
downloader_max_buffered_responses = 100
downloader_request_limit = 1000
commit_threshold = 10000

[stages.total_difficulty]
commit_threshold = 100000

[stages.bodies]
downloader_request_limit = 200
downloader_stream_batch_size = 1000
downloader_max_buffered_blocks_size_bytes = 2147483648
downloader_min_concurrent_requests = 5
downloader_max_concurrent_requests = 100

[stages.sender_recovery]
commit_threshold = 5000000

[stages.execution]
max_blocks = 500000
max_changes = 5000000
max_cumulative_gas = 1500000000000
max_duration = '10m'

[stages.account_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.storage_hashing]
clean_threshold = 500000
commit_threshold = 100000

[stages.merkle]
clean_threshold = 50000

[stages.transaction_lookup]
commit_threshold = 5000000

[stages.index_account_history]
commit_threshold = 100000

[stages.index_storage_history]
commit_threshold = 100000

[peers]
refill_slots_interval = '5s'
trusted_nodes = []
connect_trusted_nodes_only = false
max_backoff_count = 5
ban_duration = '12h'

[peers.connection_info]
max_outbound = 100
max_inbound = 30
max_concurrent_outbound_dials = 10

[peers.reputation_weights]
bad_message = -16384
bad_block = -16384
bad_transactions = -16384
already_seen_transactions = 0
timeout = -4096
bad_protocol = -2147483648
failed_to_connect = -25600
dropped = -4096
bad_announcement = -1024

[peers.backoff_durations]
low = '30s'
medium = '3m'
high = '15m'
max = '1h'

[sessions]
session_command_buffer = 32
session_event_buffer = 260

[sessions.limits]

[sessions.initial_internal_request_timeout]
secs = 20
nanos = 0

[sessions.protocol_breach_request_timeout]
secs = 120
nanos = 0
#";
        let _conf: Config = toml::from_str(alpha_0_0_19).unwrap();
    }

    #[test]
    fn test_conf_trust_nodes_only() {
        let trusted_nodes_only = r"#
[peers]
trusted_nodes_only = true
#";
        let conf: Config = toml::from_str(trusted_nodes_only).unwrap();
        assert!(conf.peers.trusted_nodes_only);

        let trusted_nodes_only = r"#
[peers]
connect_trusted_nodes_only = true
#";
        let conf: Config = toml::from_str(trusted_nodes_only).unwrap();
        assert!(conf.peers.trusted_nodes_only);
    }
}
