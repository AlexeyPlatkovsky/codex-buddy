//! Shared runtime snapshot for MCP tools.
//!
//! Runtime snapshots are process-local live state scoped by a caller-provided
//! context key. Disk is best-effort cold-start persistence; a context reads it
//! once when created and never rereads it.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use arc_swap::ArcSwapOption;
use codex_protocol::mcp::McpServerInfo;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

use self::persistence::load_cached_codex_apps_server_info;
use self::persistence::load_cached_mcp_tool_runtime_for_identity;
use self::persistence::persist_codex_apps_cache;
use self::persistence::server_info_cache_path;
use self::persistence::tools_cache_path;

const MCP_TOOLS_CACHE_PUBLISH_DURATION_METRIC: &str = "codex.mcp.tools.cache_publish.duration_ms";

/// Values stored in the MCP tool runtime's persisted snapshot.
///
/// The current persistence adapter uses the Codex Apps cache layout for every
/// serializable, cloneable payload.
pub trait McpToolRuntimePayload: Clone + Serialize + DeserializeOwned {}

impl<T> McpToolRuntimePayload for T where T: Clone + Serialize + DeserializeOwned {}

/// The account and workspace identity of an MCP tool runtime catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpToolRuntimeContextKey {
    account_id: Option<String>,
    chatgpt_user_id: Option<String>,
    is_workspace_account: bool,
}

impl McpToolRuntimeContextKey {
    pub fn personal(account_id: Option<String>, chatgpt_user_id: Option<String>) -> Self {
        Self {
            account_id,
            chatgpt_user_id,
            is_workspace_account: false,
        }
    }

    pub fn workspace(account_id: Option<String>, chatgpt_user_id: Option<String>) -> Self {
        Self {
            account_id,
            chatgpt_user_id,
            is_workspace_account: true,
        }
    }
}

pub(crate) fn mcp_tool_runtime_cache_path(
    codex_home: &Path,
    key: McpToolRuntimeContextKey,
) -> PathBuf {
    let identity = McpToolRuntimeIdentity {
        codex_home: codex_home.to_path_buf(),
        key,
    };
    tools_cache_path(&identity)
}

/// One atomically published MCP tool runtime state.
///
/// Tools remain raw and in response order. Local and managed configuration is
/// intentionally applied by readers rather than persisted in this snapshot.
#[derive(Debug, Clone)]
pub struct McpToolRuntimeSnapshot<T> {
    tools: Vec<T>,
    refreshed_at: SystemTime,
}

impl<T> McpToolRuntimeSnapshot<T> {
    pub fn tools(&self) -> &[T] {
        &self.tools
    }

    pub fn refreshed_at(&self) -> SystemTime {
        self.refreshed_at
    }

    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.refreshed_at)
            .unwrap_or_default()
    }
}

/// Process-scoped registry of MCP tool runtime state by account and workspace.
///
/// Contexts with the same identity share one live entry. Different identities
/// remain independently available for clients that already hold their context.
pub struct McpToolRuntimeManager<T: McpToolRuntimePayload> {
    entries: Arc<Mutex<HashMap<McpToolRuntimeIdentity, Arc<McpToolRuntimeEntry<T>>>>>,
    disk_cache: McpToolRuntimeDiskCache,
}

impl<T: McpToolRuntimePayload> Clone for McpToolRuntimeManager<T> {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            disk_cache: self.disk_cache,
        }
    }
}

impl<T: McpToolRuntimePayload> Default for McpToolRuntimeManager<T> {
    fn default() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            disk_cache: McpToolRuntimeDiskCache::Enabled,
        }
    }
}

impl<T: McpToolRuntimePayload> McpToolRuntimeManager<T> {
    /// Constructs a process-local MCP tool runtime that never reads or writes the disk cache.
    pub fn new_without_cache() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            disk_cache: McpToolRuntimeDiskCache::Disabled,
        }
    }

    pub fn current_snapshot(
        &self,
        codex_home: PathBuf,
        key: McpToolRuntimeContextKey,
    ) -> Option<Arc<McpToolRuntimeSnapshot<T>>> {
        self.context(codex_home, key).current_snapshot()
    }

    pub fn context(
        &self,
        codex_home: PathBuf,
        key: McpToolRuntimeContextKey,
    ) -> McpToolRuntimeContext<T> {
        let identity = McpToolRuntimeIdentity { codex_home, key };
        let mut entries = lock_unpoisoned(&self.entries);
        let entry = entries
            .entry(identity.clone())
            .or_insert_with(|| Arc::new(McpToolRuntimeEntry::new(identity, self.disk_cache)))
            .clone();
        McpToolRuntimeContext { entry }
    }
}

/// Handle to one shared account/workspace MCP tool runtime.
pub struct McpToolRuntimeContext<T: McpToolRuntimePayload> {
    entry: Arc<McpToolRuntimeEntry<T>>,
}

impl<T: McpToolRuntimePayload> Clone for McpToolRuntimeContext<T> {
    fn clone(&self) -> Self {
        Self {
            entry: Arc::clone(&self.entry),
        }
    }
}

impl<T: McpToolRuntimePayload> McpToolRuntimeContext<T> {
    pub fn current_snapshot(&self) -> Option<Arc<McpToolRuntimeSnapshot<T>>> {
        self.entry.current_snapshot.load_full()
    }

    pub fn has_current_tools(&self) -> bool {
        self.current_snapshot().is_some()
    }

    pub fn begin_fetch(&self, source: McpToolRuntimeFetchSource) -> McpToolRuntimeFetchTicket {
        McpToolRuntimeFetchTicket {
            generation: self
                .entry
                .next_fetch_generation
                .fetch_add(1, Ordering::Relaxed)
                + 1,
            source,
        }
    }

    pub fn cached_server_info(&self) -> Option<McpServerInfo> {
        match self.entry.disk_cache {
            McpToolRuntimeDiskCache::Enabled => load_cached_codex_apps_server_info(self),
            McpToolRuntimeDiskCache::Disabled => None,
        }
    }

    fn tools_cache_path(&self) -> PathBuf {
        tools_cache_path(&self.entry.identity)
    }

    fn server_info_cache_path(&self) -> PathBuf {
        server_info_cache_path(&self.entry.identity)
    }

    pub fn current_tools(&self) -> Option<Vec<T>> {
        self.current_snapshot()
            .map(|snapshot| snapshot.tools.clone())
    }

    pub fn publish_runtime_if_newest_accepted(
        &self,
        ticket: McpToolRuntimeFetchTicket,
        server_info: &McpServerInfo,
        tools: Vec<T>,
    ) -> Arc<McpToolRuntimeSnapshot<T>> {
        match self.entry.disk_cache {
            McpToolRuntimeDiskCache::Enabled => self.publish_runtime_if_newest_accepted_with(
                ticket,
                server_info,
                tools,
                persist_codex_apps_cache,
            ),
            McpToolRuntimeDiskCache::Disabled => self.publish_runtime_if_newest_accepted_with(
                ticket,
                server_info,
                tools,
                |_, _, _| {},
            ),
        }
    }

    fn publish_runtime_if_newest_accepted_with(
        &self,
        ticket: McpToolRuntimeFetchTicket,
        server_info: &McpServerInfo,
        tools: Vec<T>,
        persist: impl FnOnce(&McpToolRuntimeContext<T>, &McpServerInfo, &McpToolRuntimeSnapshot<T>),
    ) -> Arc<McpToolRuntimeSnapshot<T>> {
        let publish_start = Instant::now();
        let mut last_accepted_generation = lock_unpoisoned(&self.entry.last_accepted_generation);
        if ticket.generation <= *last_accepted_generation
            && let Some(snapshot) = self.current_snapshot()
        {
            drop(last_accepted_generation);
            emit_duration(
                MCP_TOOLS_CACHE_PUBLISH_DURATION_METRIC,
                publish_start.elapsed(),
                &[("source", ticket.source.as_str()), ("result", "stale")],
            );
            return snapshot;
        }

        let snapshot = Arc::new(McpToolRuntimeSnapshot {
            tools,
            refreshed_at: SystemTime::now(),
        });

        *last_accepted_generation = ticket.generation;
        self.entry
            .current_snapshot
            .store(Some(Arc::clone(&snapshot)));
        // Keep the generation guard through persistence so accepted generations cannot reach disk
        // out of order.
        persist(self, server_info, snapshot.as_ref());
        drop(last_accepted_generation);
        emit_duration(
            MCP_TOOLS_CACHE_PUBLISH_DURATION_METRIC,
            publish_start.elapsed(),
            &[("source", ticket.source.as_str()), ("result", "published")],
        );
        snapshot
    }

    pub fn publish_if_newest_accepted(
        &self,
        ticket: McpToolRuntimeFetchTicket,
        server_info: &McpServerInfo,
        tools: Vec<T>,
    ) -> Vec<T> {
        self.publish_runtime_if_newest_accepted(ticket, server_info, tools)
            .tools
            .clone()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum McpToolRuntimeFetchSource {
    Startup,
    HardRefresh,
}

impl McpToolRuntimeFetchSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::HardRefresh => "hard_refresh",
        }
    }
}

pub struct McpToolRuntimeFetchTicket {
    generation: u64,
    source: McpToolRuntimeFetchSource,
}

/// All live state owned by one MCP tool runtime identity.
struct McpToolRuntimeEntry<T: McpToolRuntimePayload> {
    identity: McpToolRuntimeIdentity,
    disk_cache: McpToolRuntimeDiskCache,
    current_snapshot: ArcSwapOption<McpToolRuntimeSnapshot<T>>,
    next_fetch_generation: AtomicU64,
    last_accepted_generation: Mutex<u64>,
}

impl<T: McpToolRuntimePayload> McpToolRuntimeEntry<T> {
    fn new(identity: McpToolRuntimeIdentity, disk_cache: McpToolRuntimeDiskCache) -> Self {
        let current_snapshot = match disk_cache {
            McpToolRuntimeDiskCache::Enabled => {
                load_cached_mcp_tool_runtime_for_identity(&identity).map(Arc::new)
            }
            McpToolRuntimeDiskCache::Disabled => None,
        };
        Self {
            identity,
            disk_cache,
            current_snapshot: ArcSwapOption::from(current_snapshot),
            next_fetch_generation: AtomicU64::new(0),
            last_accepted_generation: Mutex::new(0),
        }
    }
}

#[derive(Clone, Copy)]
enum McpToolRuntimeDiskCache {
    Enabled,
    Disabled,
}

/// Everything that decides whether two MCP tool runtime clients can share a snapshot.
///
/// The auth key says whose runtime catalog we are reading. `codex_home` keeps
/// the persisted cache under the right home directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct McpToolRuntimeIdentity {
    codex_home: PathBuf,
    key: McpToolRuntimeContextKey,
}

fn emit_duration(metric: &str, duration: Duration, tags: &[(&str, &str)]) {
    if let Some(metrics) = codex_otel::global() {
        let _ = metrics.record_duration(metric, duration, tags);
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

mod persistence;

#[cfg(test)]
mod tests;
