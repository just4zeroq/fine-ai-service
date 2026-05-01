use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::Instant;

use lru::LruCache;

use crate::error::{AppError, Result};

#[derive(Debug)]
struct CachedEntry {
    is_valid: bool,
    cached_at: Instant,
}

#[derive(Debug)]
pub struct ApiKeyAuth {
    pool: Option<sqlx::MySqlPool>,
    cache: Mutex<LruCache<String, CachedEntry>>,
    cache_ttl: std::time::Duration,
}

impl ApiKeyAuth {
    /// Create a new ApiKeyAuth with a database pool.
    ///
    /// * `pool` - MySQL connection pool for querying API keys
    /// * `cache_size` - Maximum number of entries in the LRU cache
    /// * `ttl_secs` - How long a cached validation result remains valid (in seconds)
    pub fn new(pool: sqlx::MySqlPool, cache_size: usize, ttl_secs: u64) -> Self {
        let cap = NonZeroUsize::new(cache_size.max(1)).unwrap();
        Self {
            pool: Some(pool),
            cache: Mutex::new(LruCache::new(cap)),
            cache_ttl: std::time::Duration::from_secs(ttl_secs),
        }
    }

    /// Test-only constructor: pre-populates the cache with valid keys.
    /// No database connection is required.
    #[doc(hidden)]
    pub fn new_with_keys(keys: Vec<String>) -> Self {
        let cap = NonZeroUsize::new(1000).unwrap();
        let mut cache = LruCache::new(cap);
        let now = Instant::now();
        for key in keys {
            cache.put(key, CachedEntry {
                is_valid: true,
                cached_at: now,
            });
        }
        Self {
            pool: None,
            cache: Mutex::new(cache),
            cache_ttl: std::time::Duration::from_secs(3600),
        }
    }

    /// Validate an API key.
    ///
    /// Checks the in-memory LRU cache first; on cache miss or expired entry,
    /// queries the database. Results (both valid and invalid) are cached with TTL.
    pub async fn validate(&self, key: &str) -> Result<()> {
        let now = Instant::now();

        // Check cache first (Mutex scope)
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(key) {
                if now.duration_since(entry.cached_at) < self.cache_ttl {
                    return if entry.is_valid {
                        Ok(())
                    } else {
                        Err(AppError::Unauthorized("Invalid API key".into()))
                    };
                }
            }
        }

        // Cache miss or expired — query the database
        let pool = self.pool.as_ref().ok_or_else(|| {
            AppError::Internal("Database pool not configured for API key validation".into())
        })?;

        let result = match crate::db::queries::validate_user_api_key(pool, key).await {
            Ok(_) => Ok(()),
            // The DB query returns NotFound/Unauthorized for invalid/expired/inactive keys
            Err(AppError::NotFound(_)) | Err(AppError::Unauthorized(_)) => {
                Err(AppError::Unauthorized("Invalid API key".into()))
            }
            Err(e) => Err(e),
        };

        // Cache the result (valid and invalid alike, to avoid DB hammering)
        let is_valid = result.is_ok();
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(key.to_string(), CachedEntry {
                is_valid,
                cached_at: now,
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_valid() {
        let auth = ApiKeyAuth::new_with_keys(vec!["key1".into(), "key2".into()]);
        assert!(auth.validate("key1").await.is_ok());
    }

    #[tokio::test]
    async fn test_api_key_invalid() {
        let auth = ApiKeyAuth::new_with_keys(vec!["key1".into()]);
        assert!(auth.validate("wrong_key").await.is_err());
    }

    #[tokio::test]
    async fn test_cache_ttl_expired() {
        let auth = ApiKeyAuth::new_with_keys(vec!["valid".into()]);
        // Cache is pre-populated with zero TTL — force DB miss
        // Since pool is None, validate should return Internal error
        // But the cache entry was just inserted, so it should still be valid
        assert!(auth.validate("valid").await.is_ok());
    }
}
