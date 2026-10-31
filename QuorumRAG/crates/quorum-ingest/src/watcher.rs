//! Continuous Real-Time OS Filesystem Watcher with 300ms Debouncing.
//!
//! Monitors designated filesystem roots for creation, modification, deletion,
//! and rename events, debouncing rapid bursts into normalized `WatcherEvent` items.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::IngestConfig;
use crate::error::IngestError;

/// Normalized debounced filesystem event emitted by the OS watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatcherEvent {
    /// Newly created file.
    Create(PathBuf),
    /// Content or metadata modified.
    Modify(PathBuf),
    /// File removed from disk.
    Remove(PathBuf),
    /// File renamed or moved from old path to new path.
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    /// Event queue overflow or dropped events -> requires drift healing scan.
    QueueOverflow {
        root: PathBuf,
        details: String,
    },
}

/// Continuous real-time OS filesystem watcher with 300ms debouncing.
pub struct FilesystemWatcher {
    config: IngestConfig,
    active_paths: HashSet<PathBuf>,
}

impl FilesystemWatcher {
    pub fn new(config: IngestConfig) -> Self {
        Self {
            config,
            active_paths: HashSet::new(),
        }
    }

    /// Spawns the continuous OS watcher with a 300ms debouncing window.
    ///
    /// Returns:
    /// - `mpsc::Receiver<WatcherEvent>`: Non-blocking event receiver.
    /// - `WatcherHandle`: Control handle for dynamic watch management and clean shutdown.
    pub fn start_watching(
        mut self,
    ) -> Result<(mpsc::Receiver<WatcherEvent>, WatcherHandle), IngestError> {
        let (tx, rx) = mpsc::channel(10_000);
        let debounce_duration = self.config.debounce_duration();

        let tx_event = tx.clone();
        let mut debouncer = new_debouncer(
            debounce_duration,
            None,
            move |res: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match res {
                    Ok(events) => {
                        for event in events {
                            Self::dispatch_debounced_event(&event, &tx_event);
                        }
                    }
                    Err(errors) => {
                        for err in errors {
                            warn!("OS Watcher native error: {}", err);
                            let _ = tx_event.try_send(WatcherEvent::QueueOverflow {
                                root: PathBuf::new(),
                                details: err.to_string(),
                            });
                        }
                    }
                }
            },
        )
        .map_err(|e| IngestError::WatcherError(format!("Failed to initialize debouncer: {}", e)))?;

        // Register initial watch paths recursively
        for path in &self.config.watch_paths {
            if path.exists() {
                debouncer
                    .watcher()
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| {
                        IngestError::WatcherError(format!(
                            "Failed to watch path '{}': {}",
                            path.display(),
                            e
                        ))
                    })?;
                self.active_paths.insert(path.clone());
                info!(path = %path.display(), "Watching directory recursively");
            } else {
                warn!("Watch directory does not exist, skipping: {}", path.display());
            }
        }

        let handle = WatcherHandle {
            debouncer: Arc::new(std::sync::Mutex::new(debouncer)),
            active_paths: Arc::new(std::sync::RwLock::new(self.active_paths)),
        };

        Ok((rx, handle))
    }

    /// Maps native `DebouncedEvent` into QuorumRAG `WatcherEvent`.
    fn dispatch_debounced_event(event: &DebouncedEvent, tx: &mpsc::Sender<WatcherEvent>) {
        use notify::event::ModifyKind;

        match event.kind {
            EventKind::Create(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        let _ = tx.try_send(WatcherEvent::Create(path.clone()));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                // Rename or move event
                if event.paths.len() >= 2 {
                    let from = event.paths[0].clone();
                    let to = event.paths[1].clone();
                    let _ = tx.try_send(WatcherEvent::Rename { from, to });
                } else if let Some(path) = event.paths.first() {
                    // Single-path rename (e.g. rename from outside watch dir)
                    if path.exists() {
                        let _ = tx.try_send(WatcherEvent::Create(path.clone()));
                    } else {
                        let _ = tx.try_send(WatcherEvent::Remove(path.clone()));
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in &event.paths {
                    if path.is_file() {
                        let _ = tx.try_send(WatcherEvent::Modify(path.clone()));
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in &event.paths {
                    let _ = tx.try_send(WatcherEvent::Remove(path.clone()));
                }
            }
            EventKind::Other => {
                debug!("Received non-actionable watcher event");
            }
            _ => {}
        }
    }
}

/// Control handle for dynamic watch management and thread-safe operations.
pub struct WatcherHandle {
    debouncer: Arc<std::sync::Mutex<Debouncer<RecommendedWatcher, FileIdMap>>>,
    active_paths: Arc<std::sync::RwLock<HashSet<PathBuf>>>,
}

impl WatcherHandle {
    /// Dynamically registers a new directory root for real-time monitoring.
    pub fn watch_path(&self, path: impl AsRef<Path>) -> Result<(), IngestError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(IngestError::WatcherError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        let mut debouncer = self.debouncer.lock().map_err(|_| {
            IngestError::WatcherError("Debouncer mutex poisoned".into())
        })?;

        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| {
                IngestError::WatcherError(format!(
                    "Failed to register watch for '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        let mut paths = self.active_paths.write().unwrap();
        paths.insert(path);
        Ok(())
    }

    /// Dynamically unregisters an active directory root.
    pub fn unwatch_path(&self, path: impl AsRef<Path>) -> Result<(), IngestError> {
        let path = path.as_ref();
        let mut debouncer = self.debouncer.lock().map_err(|_| {
            IngestError::WatcherError("Debouncer mutex poisoned".into())
        })?;

        debouncer
            .watcher()
            .unwatch(path)
            .map_err(|e| {
                IngestError::WatcherError(format!(
                    "Failed to unwatch path '{}': {}",
                    path.display(),
                    e
                ))
            })?;

        let mut paths = self.active_paths.write().unwrap();
        paths.remove(path);
        Ok(())
    }

    /// Returns the snapshot list of all actively monitored filesystem roots.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let paths = self.active_paths.read().unwrap();
        paths.iter().cloned().collect()
    }
}
