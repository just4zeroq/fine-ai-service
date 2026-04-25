use stock_gateway::rate_limit::{RateLimiter, RateLimitResult};
use stock_gateway::config::RateLimitConfig;

#[test]
fn test_ip_rate_limit() {
    let cfg = RateLimitConfig {
        ip_limit: 3,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    // First 3 should pass
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    // 4th should fail
    assert_eq!(limiter.check_ip("192.168.1.1"), false);
}

#[test]
fn test_different_ips_independent() {
    let cfg = RateLimitConfig {
        ip_limit: 1,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    assert!(limiter.check_ip("192.168.1.1"));
    assert!(!limiter.check_ip("192.168.1.1"));
    assert!(limiter.check_ip("192.168.1.2")); // Different IP, should pass
}

#[test]
fn test_key_rate_limit() {
    let cfg = RateLimitConfig {
        ip_limit: 100,
        key_limit: 3,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    // First 3 with key should pass
    assert_eq!(limiter.check_key("apikey1"), true);
    assert_eq!(limiter.check_key("apikey1"), true);
    assert_eq!(limiter.check_key("apikey1"), true);
    // 4th should fail
    assert_eq!(limiter.check_key("apikey1"), false);
}

#[test]
fn test_combined_check_with_key() {
    let cfg = RateLimitConfig {
        ip_limit: 1,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    // With key, should use key limit (not IP limit)
    assert_eq!(limiter.check("192.168.1.1", Some("apikey1")), RateLimitResult::Allowed);
    assert_eq!(limiter.check("192.168.1.1", Some("apikey1")), RateLimitResult::Allowed);
}

#[test]
fn test_combined_check_without_key() {
    let cfg = RateLimitConfig {
        ip_limit: 2,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    // Without key, should use IP limit
    assert_eq!(limiter.check("192.168.1.1", None), RateLimitResult::Allowed);
    assert_eq!(limiter.check("192.168.1.1", None), RateLimitResult::Allowed);
    assert_eq!(limiter.check("192.168.1.1", None), RateLimitResult::IpLimited);
}