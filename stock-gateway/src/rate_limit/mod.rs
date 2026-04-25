pub mod sliding_window;

pub use sliding_window::SlidingWindowLimiter;

use crate::config::RateLimitConfig;

pub struct RateLimiter {
    ip_limiter: SlidingWindowLimiter,
    key_limiter: SlidingWindowLimiter,
}

impl RateLimiter {
    pub fn new(cfg: &RateLimitConfig) -> Self {
        Self {
            ip_limiter: SlidingWindowLimiter::new(cfg.ip_limit, cfg.window_sec),
            key_limiter: SlidingWindowLimiter::new(cfg.key_limit, cfg.window_sec),
        }
    }

    /// Check IP rate limit (Layer 1)
    pub fn check_ip(&self, ip: &str) -> bool {
        self.ip_limiter.check(ip)
    }

    /// Check Key rate limit (Layer 2)
    pub fn check_key(&self, key: &str) -> bool {
        self.key_limiter.check(key)
    }

    /// Check combined: if key present use key limit, else use IP limit
    pub fn check(&self, ip: &str, key: Option<&str>) -> RateLimitResult {
        if let Some(k) = key {
            if self.key_limiter.check(k) {
                RateLimitResult::Allowed
            } else {
                RateLimitResult::KeyLimited
            }
        } else if self.ip_limiter.check(ip) {
            RateLimitResult::Allowed
        } else {
            RateLimitResult::IpLimited
        }
    }

    pub fn ip_limiter(&self) -> &SlidingWindowLimiter {
        &self.ip_limiter
    }

    pub fn key_limiter(&self) -> &SlidingWindowLimiter {
        &self.key_limiter
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateLimitResult {
    Allowed,
    IpLimited,
    KeyLimited,
}