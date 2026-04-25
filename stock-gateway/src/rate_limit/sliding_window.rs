use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone)]
struct Window {
    count: std::sync::Arc<AtomicU64>,
    start: Instant,
}

pub struct SlidingWindowLimiter {
    limit: u32,
    window_sec: u64,
    counters: DashMap<String, Window>,
}

impl SlidingWindowLimiter {
    pub fn new(limit: u32, window_sec: u64) -> Self {
        Self {
            limit,
            window_sec,
            counters: DashMap::new(),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();

        let entry = self.counters.entry(key.to_string()).or_insert_with(|| Window {
            count: std::sync::Arc::new(AtomicU64::new(0)),
            start: now,
        });

        // Reset if window expired
        let elapsed = now.duration_since(entry.start).as_secs();
        if elapsed >= self.window_sec {
            entry.count.store(0, Ordering::SeqCst);
            // Note: Can't update start in DashMap entry directly, recreate entry
            self.counters.remove(key);
            let new_entry = self.counters.entry(key.to_string()).or_insert_with(|| Window {
                count: std::sync::Arc::new(AtomicU64::new(0)),
                start: now,
            });
            new_entry.count.fetch_add(1, Ordering::SeqCst);
            return true;
        }

        let current = entry.count.load(Ordering::SeqCst);
        if current >= self.limit as u64 {
            return false;
        }
        entry.count.fetch_add(1, Ordering::SeqCst);
        true
    }

    pub fn remaining(&self, key: &str) -> u32 {
        let now = Instant::now();
        let entry = match self.counters.get(key) {
            Some(e) => e,
            None => return self.limit,
        };

        let elapsed = now.duration_since(entry.start).as_secs();
        if elapsed >= self.window_sec {
            return self.limit;
        }

        let current = entry.count.load(Ordering::SeqCst);
        self.limit.saturating_sub(current as u32)
    }
}