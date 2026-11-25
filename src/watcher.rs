use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tao::event_loop::EventLoopProxy;

use crate::UserEvent;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    ref_counts: HashMap<PathBuf, usize>,
    last_emit: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl FileWatcher {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> notify::Result<Self> {
        let last_emit: Arc<Mutex<HashMap<PathBuf, Instant>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let last_emit_cb = Arc::clone(&last_emit);

        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            let dominated = matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_)
            );
            if !dominated {
                return;
            }
            let now = Instant::now();
            for path in event.paths {
                let Ok(mut map) = last_emit_cb.lock() else {
                    continue;
                };
                if let Some(prev) = map.get(&path)
                    && now.duration_since(*prev) < Duration::from_millis(180)
                {
                    continue;
                }
                map.insert(path.clone(), now);
                drop(map);
                let _ = proxy.send_event(UserEvent::FileChanged(path));
            }
        })?;

        Ok(Self {
            watcher,
            ref_counts: HashMap::new(),
            last_emit,
        })
    }

    pub fn watch(&mut self, path: &Path) {
        let canonical = path.to_path_buf();
        let count = self.ref_counts.entry(canonical.clone()).or_insert(0);
        if *count == 0 {
            let _ = self.watcher.watch(&canonical, RecursiveMode::NonRecursive);
        }
        *count += 1;
    }

    pub fn unwatch(&mut self, path: &Path) {
        let canonical = path.to_path_buf();
        if let Some(count) = self.ref_counts.get_mut(&canonical) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.ref_counts.remove(&canonical);
                let _ = self.watcher.unwatch(&canonical);
                if let Ok(mut map) = self.last_emit.lock() {
                    map.remove(&canonical);
                }
            }
        }
    }
}
