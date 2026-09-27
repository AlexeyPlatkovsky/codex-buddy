//! Shared runtime snapshot for MCP tools.
//!
//! Runtime snapshots are process-local live state scoped by a caller-provided
//! context key. Disk is best-effort cold-start persistence; a context reads it
//! once when created and never rereads it. Full connector metadata is
//! owned by the connector metadata store, not by this module.
//! Live catalog subscriptions publish only successful fetches from a matching
//! discovery scope, never disk snapshots or another scope's discovery winner.
//! Equivalent live contexts share one current catalog. Each fetch still contacts
//! the server; equal definitions retain their established storage and ordering.
//! Providers expire with their last context.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
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
use tokio::sync::watch;

use self::persistence::load_cached_codex_apps_server_info;
use self::persistence::load_cached_mcp_tool_runtime_for_identity;
use self::persistence::persist_codex_apps_cache;
use self::persistence::server_info_cache_path;
use self::persistence::tools_cache_path;

const MCP_TOOLS_CACHE_PUBLISH_DURATION_METRIC: &str = "codex.mcp.tools.cache_publish.duration_ms";

/// The current immutable tools for matching discovery inputs.
struct CatalogProvider<T> {
    scope: Vec<String>,
    updates: watch::Sender<Option<Arc<McpToolRuntimeSnapshot<T>>>>,
}

/// Values stored in the MCP tool runtime's persisted tool snapshot.
///
/// The runtime uses the Codex Apps cache layout for every
/// serializable, cloneable payload. Equality determines whether fresh results can
/// retain the previous storage and tool version, so it must include all metadata
/// that affects readers or prepared calls.
pub trait McpToolRuntimePayload: Clone + PartialEq + Serialize + DeserializeOwned {}

impl<T> McpToolRuntimePayload for T where T: Clone + PartialEq + Serialize + DeserializeOwned {}

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
    tools: Arc<[T]>,
    refreshed_at: SystemTime,
    generation: u64,
    tools_version: u64,
}

impl<T> McpToolRuntimeSnapshot<T> {
    pub fn tools(&self) -> &[T] {
        &self.tools
    }

    pub fn shared_tools(&self) -> Arc<[T]> {
        Arc::clone(&self.tools)
    }

    /// Advances only when tool definitions change, so equal refreshes preserve prepared calls.
    pub fn tools_version(&self) -> u64 {
        self.tools_version
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
        McpToolRuntimeContext {
            entry,
            live_catalog: None,
        }
    }
}

/// Handle to one shared account/workspace MCP tool runtime.
pub struct McpToolRuntimeContext<T: McpToolRuntimePayload> {
    entry: Arc<McpToolRuntimeEntry<T>>,
    live_catalog: Option<Arc<CatalogProvider<T>>>,
}

impl<T: McpToolRuntimePayload> Clone for McpToolRuntimeContext<T> {
    fn clone(&self) -> Self {
        Self {
            entry: Arc::clone(&self.entry),
            live_catalog: self.live_catalog.clone(),
        }
    }
}

impl<T: McpToolRuntimePayload> McpToolRuntimeContext<T> {
    /// Groups executable catalogs by equivalent discovery inputs within this account/home.
    /// Include transport/auth, requested capabilities, and initialization results.
    /// Calling this again refines the existing scope; it does not replace it.
    /// Account-wide discovery reads are unaffected.
    pub fn with_live_scope(mut self, scope: String) -> Self {
        let mut scope_parts = self
            .live_catalog
            .as_ref()
            .map(|catalog| catalog.scope.clone())
            .unwrap_or_default();
        scope_parts.push(scope);
        let mut catalogs = lock_unpoisoned(&self.entry.live_catalogs);
        catalogs.retain(|_, catalog| catalog.strong_count() > 0);
        let catalog = catalogs.entry(scope_parts.clone()).or_default();
        self.live_catalog = Some(catalog.upgrade().unwrap_or_else(|| {
            let provider = Arc::new(CatalogProvider {
                scope: scope_parts,
                updates: watch::channel(None).0,
            });
            *catalog = Arc::downgrade(&provider);
            provider
        }));
        drop(catalogs);
        self
    }

    /// Detaches a server that opts out of live catalog sharing, including publication.
    pub fn without_live_scope(mut self) -> Self {
        self.live_catalog = None;
        self
    }

    /// Subscribes to accepted live tools without refetching or waiting for other clients.
    pub fn subscribe(&self) -> Option<watch::Receiver<Option<Arc<McpToolRuntimeSnapshot<T>>>>> {
        self.live_catalog
            .as_ref()
            .map(|catalog| catalog.updates.subscribe())
    }

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
            .map(|snapshot| snapshot.tools.to_vec())
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
        let prior = self
            .live_catalog
            .as_ref()
            .and_then(|catalog| catalog.updates.borrow().clone());
        let (tools, tools_version) = match prior {
            Some(prior)
                if prior.generation < ticket.generation && prior.tools() == tools.as_slice() =>
            {
                (prior.shared_tools(), prior.tools_version)
            }
            _ => (tools.into(), ticket.generation),
        };
        let snapshot = Arc::new(McpToolRuntimeSnapshot {
            tools,
            tools_version,
            refreshed_at: SystemTime::now(),
            generation: ticket.generation,
        });
        if let Some(live_catalog) = &self.live_catalog {
            live_catalog.updates.send_if_modified(|current| {
                if current
                    .as_ref()
                    .is_some_and(|current| current.generation >= ticket.generation)
                {
                    return false;
                }
                *current = Some(Arc::clone(&snapshot));
                true
            });
        }
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
            .to_vec()
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
    live_catalogs: Mutex<HashMap<Vec<String>, Weak<CatalogProvider<T>>>>,
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
            live_catalogs: Mutex::new(HashMap::new()),
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
