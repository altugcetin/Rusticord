use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::error::{RateLimitScope, seconds_to_duration};
use crate::route::{BucketKey, RestRoute};

const GLOBAL_CAPACITY: u32 = 50;
const GLOBAL_REFILL_PERIOD: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct BucketState {
    pub remaining: u32,
    pub reset_at: Instant,
}

#[derive(Debug)]
pub struct RateLimiter {
    global_tokens: u32,
    global_refill_at: Instant,
    buckets: HashMap<BucketKey, BucketState>,
    route_to_bucket: HashMap<String, String>,
}

impl RateLimiter {
    pub fn new(now: Instant) -> Self {
        Self {
            global_tokens: GLOBAL_CAPACITY,
            global_refill_at: now,
            buckets: HashMap::new(),
            route_to_bucket: HashMap::new(),
        }
    }

    pub fn delay_before_send(&mut self, route: &RestRoute, now: Instant) -> Duration {
        self.refill_global(now);
        let global_wait = if self.global_tokens == 0 {
            self.global_refill_at.saturating_duration_since(now)
        } else {
            Duration::ZERO
        };
        let bucket_wait = self
            .bucket_for(route)
            .and_then(|state| {
                if state.remaining == 0 {
                    Some(state.reset_at.saturating_duration_since(now))
                } else {
                    None
                }
            })
            .unwrap_or(Duration::ZERO);
        max_duration(global_wait, bucket_wait)
    }

    pub fn note_send(&mut self, now: Instant) {
        self.refill_global(now);
        if self.global_tokens > 0 {
            self.global_tokens -= 1;
        }
        if self.global_tokens == 0 {
            self.global_refill_at = now + GLOBAL_REFILL_PERIOD;
        }
    }

    pub fn observe_headers(&mut self, route: &RestRoute, headers: &RateLimitHeaders, now: Instant) {
        if let Some(hash) = headers.bucket.as_deref() {
            let route_id = route_identity(route);
            self.route_to_bucket.insert(route_id, String::from(hash));
            let key = BucketKey::from_header(hash, route.major);
            if let Some(remaining) = headers.remaining {
                let reset_after = headers.reset_after.unwrap_or(Duration::from_secs(1));
                self.buckets.insert(
                    key,
                    BucketState {
                        remaining,
                        reset_at: now + reset_after,
                    },
                );
            }
        }
    }

    pub fn observe_rate_limited(&mut self, route: &RestRoute, retry_after: Duration, now: Instant) {
        let key = self.bucket_key(route);
        self.buckets.insert(
            key,
            BucketState {
                remaining: 0,
                reset_at: now + retry_after,
            },
        );
        if self.global_tokens > 0 {
            self.global_tokens = self.global_tokens.saturating_sub(1);
        }
    }

    fn refill_global(&mut self, now: Instant) {
        if self.global_tokens == 0 && now >= self.global_refill_at {
            self.global_tokens = GLOBAL_CAPACITY;
            self.global_refill_at = now + GLOBAL_REFILL_PERIOD;
        }
    }

    fn bucket_for(&self, route: &RestRoute) -> Option<&BucketState> {
        self.buckets.get(&self.bucket_key(route))
    }

    fn bucket_key(&self, route: &RestRoute) -> BucketKey {
        let route_id = route_identity(route);
        match self.route_to_bucket.get(&route_id) {
            Some(hash) => BucketKey::from_header(hash, route.major),
            None => BucketKey::from_route(route),
        }
    }
}

fn route_identity(route: &RestRoute) -> String {
    format!("{:?}:{}:{:?}", route.method, route.path, route.major)
}

fn max_duration(left: Duration, right: Duration) -> Duration {
    if left > right { left } else { right }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RateLimitHeaders {
    pub bucket: Option<String>,
    pub remaining: Option<u32>,
    pub reset_after: Option<Duration>,
    pub retry_after: Option<Duration>,
    pub global: bool,
    pub scope: Option<RateLimitScope>,
}

impl RateLimitHeaders {
    pub fn parse(get: impl Fn(&str) -> Option<String>) -> Self {
        let reset_after = get("x-ratelimit-reset-after")
            .or_else(|| get("X-RateLimit-Reset-After"))
            .and_then(|value| value.parse::<f64>().ok())
            .map(seconds_to_duration);
        let retry_after = get("retry-after")
            .or_else(|| get("Retry-After"))
            .as_deref()
            .and_then(crate::error::parse_retry_after_header);
        let remaining = get("x-ratelimit-remaining")
            .or_else(|| get("X-RateLimit-Remaining"))
            .and_then(|value| value.parse().ok());
        let bucket = get("x-ratelimit-bucket").or_else(|| get("X-RateLimit-Bucket"));
        let global = get("x-ratelimit-global")
            .or_else(|| get("X-RateLimit-Global"))
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        let scope = get("x-ratelimit-scope")
            .or_else(|| get("X-RateLimit-Scope"))
            .map(|value| RateLimitScope::parse(&value));
        Self {
            bucket,
            remaining,
            reset_after,
            retry_after,
            global,
            scope,
        }
    }
}

pub fn retry_wait(retry_after: Duration, attempt: u8, jitter_per_mille: u16) -> Duration {
    let capped = attempt.min(4);
    let multiplier = 1_u32 << u32::from(capped);
    let base = retry_after.saturating_mul(multiplier);
    let jitter_ms = base
        .as_millis()
        .saturating_mul(u128::from(jitter_per_mille))
        / 1000;
    let jitter_ms = u64::try_from(jitter_ms.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);
    base.saturating_add(Duration::from_millis(jitter_ms))
}

#[cfg(test)]
mod tests {
    use super::{RateLimitHeaders, RateLimiter, retry_wait};
    use crate::route::RestRoute;
    use rusticord_core::Snowflake;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    #[test]
    fn waits_when_bucket_remaining_is_zero() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new(now);
        let channel = Snowflake::from_raw(99);
        let route = RestRoute::channel_messages(channel);
        let mut headers = HashMap::new();
        headers.insert("x-ratelimit-bucket", "abcd");
        headers.insert("x-ratelimit-remaining", "0");
        headers.insert("x-ratelimit-reset-after", "2");
        limiter.observe_headers(
            &route,
            &RateLimitHeaders::parse(|key| headers.get(key).map(|value| String::from(*value))),
            now,
        );
        let wait = limiter.delay_before_send(&route, now);
        assert_eq!(wait, Duration::from_secs(2));
    }

    #[test]
    fn different_majors_do_not_share_a_bucket() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new(now);
        let first = RestRoute::channel_messages(Snowflake::from_raw(1));
        let second = RestRoute::channel_messages(Snowflake::from_raw(2));
        let mut headers = HashMap::new();
        headers.insert("x-ratelimit-bucket", "shared");
        headers.insert("x-ratelimit-remaining", "0");
        headers.insert("x-ratelimit-reset-after", "5");
        limiter.observe_headers(
            &first,
            &RateLimitHeaders::parse(|key| headers.get(key).map(|value| String::from(*value))),
            now,
        );
        let wait = limiter.delay_before_send(&second, now);
        assert_eq!(wait, Duration::ZERO);
    }

    #[test]
    fn exponential_backoff_doubles_then_adds_jitter() {
        let base = Duration::from_secs(1);
        assert_eq!(retry_wait(base, 0, 0), Duration::from_secs(1));
        assert_eq!(retry_wait(base, 1, 0), Duration::from_secs(2));
        assert_eq!(retry_wait(base, 2, 0), Duration::from_secs(4));
        assert_eq!(retry_wait(base, 3, 250), Duration::from_millis(8000 + 2000));
    }

    #[test]
    fn global_tokens_exhaust_and_wait() {
        let now = Instant::now();
        let mut limiter = RateLimiter::new(now);
        for _ in 0..50 {
            limiter.note_send(now);
        }
        let wait = limiter.delay_before_send(&RestRoute::current_user(), now);
        assert!(wait > Duration::ZERO);
    }
}
