use axum::http::{HeaderMap, Method};
use loco_rs::environment::Environment;
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BUCKETS: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Rule {
    General,
    DataExport,
    SyncBatch,
    MedicationLookup,
    AiSuggestion,
    AdminAudit,
}

impl Rule {
    fn limit(self) -> u32 {
        match self {
            Self::General => 300,
            Self::DataExport | Self::AiSuggestion => 10,
            Self::SyncBatch => 30,
            Self::MedicationLookup => 60,
            Self::AdminAudit => 100,
        }
    }

    fn period(self) -> u64 {
        if self == Self::General { 300 } else { 60 }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Key {
    ip: IpAddr,
    rule: Rule,
}

#[derive(Clone, Copy)]
struct Bucket {
    expires_at: u64,
    count: u32,
}

#[derive(Default)]
struct Counters {
    buckets: HashMap<Key, Bucket>,
    expiry: BinaryHeap<Reverse<(u64, Key)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub limit: u32,
    pub retry_after: u64,
    pub reset_at: u64,
}

struct RateLimiter {
    counters: Mutex<Counters>,
    capacity: usize,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            counters: Mutex::new(Counters::default()),
            capacity: MAX_BUCKETS,
        }
    }
}

impl RateLimiter {
    pub fn check(&self, ip: IpAddr, method: &Method, path: &str, now: u64) -> Option<Rejection> {
        let mut counters = self
            .counters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while let Some(Reverse((expires_at, key))) = counters.expiry.peek().copied() {
            if expires_at > now {
                break;
            }
            counters.expiry.pop();
            if counters
                .buckets
                .get(&key)
                .is_some_and(|bucket| bucket.expires_at == expires_at)
            {
                counters.buckets.remove(&key);
            }
        }
        for rule in [Some(Rule::General), operation_rule(method, path)]
            .into_iter()
            .flatten()
        {
            let key = Key { ip, rule };
            let period = rule.period();
            let reset_at = (now / period + 1) * period;
            if !counters.buckets.contains_key(&key) {
                if counters.buckets.len() >= self.capacity {
                    return Some(Rejection {
                        limit: rule.limit(),
                        retry_after: reset_at - now,
                        reset_at,
                    });
                }
                counters.buckets.insert(
                    key,
                    Bucket {
                        expires_at: reset_at,
                        count: 0,
                    },
                );
                counters.expiry.push(Reverse((reset_at, key)));
            }
            let bucket = counters.buckets.get_mut(&key).expect("inserted bucket");
            bucket.count = bucket.count.saturating_add(1);
            if bucket.count > rule.limit() {
                return Some(Rejection {
                    limit: rule.limit(),
                    retry_after: reset_at - now,
                    reset_at,
                });
            }
        }
        None
    }

    #[cfg(test)]
    fn bucket_count(&self) -> usize {
        self.counters.lock().unwrap().buckets.len()
    }

    #[cfg(test)]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            counters: Mutex::new(Counters::default()),
            capacity,
        }
    }
}

pub struct RateLimitState {
    limiter: RateLimiter,
    trusted: HashSet<IpAddr>,
    production: bool,
    fixed_now: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RateLimitConfigError {
    InvalidSettings,
    InvalidTrustedProxyList,
    InvalidTrustedProxyIp,
    InvalidFixedClock,
    FixedClockOutsideTest,
}

impl std::fmt::Display for RateLimitConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSettings => "API rate-limit settings must be an object",
            Self::InvalidTrustedProxyList => {
                "Trusted proxy IPs must be a comma-separated string with no empty members"
            }
            Self::InvalidTrustedProxyIp => "Invalid trusted proxy IP",
            Self::InvalidFixedClock => "Fixed rate-limit clock must be a nonnegative integer",
            Self::FixedClockOutsideTest => "Fixed rate-limit clock is only available in test",
        })
    }
}

impl std::error::Error for RateLimitConfigError {}

impl RateLimitState {
    pub fn from_settings(
        settings: Option<&Value>,
        environment: &Environment,
    ) -> Result<Self, RateLimitConfigError> {
        let configuration = settings.and_then(|settings| settings.get("api_rate_limit"));
        if configuration.is_some_and(|value| !value.is_object()) {
            return Err(RateLimitConfigError::InvalidSettings);
        }
        let trusted = match configuration.and_then(|value| value.get("trusted_proxy_ips")) {
            None => HashSet::new(),
            Some(Value::String(value)) if value.trim().is_empty() => HashSet::new(),
            Some(Value::String(value)) => value
                .split(',')
                .map(|part| {
                    if part.trim().is_empty() {
                        return Err(RateLimitConfigError::InvalidTrustedProxyList);
                    }
                    part.trim()
                        .parse::<IpAddr>()
                        .map_err(|_| RateLimitConfigError::InvalidTrustedProxyIp)
                })
                .collect::<Result<HashSet<_>, _>>()?,
            Some(_) => return Err(RateLimitConfigError::InvalidTrustedProxyList),
        };
        let fixed_now = match configuration.and_then(|value| value.get("fixed_now_unix_seconds")) {
            None => None,
            Some(value) if *environment == Environment::Test => Some(
                value
                    .as_u64()
                    .ok_or(RateLimitConfigError::InvalidFixedClock)?,
            ),
            Some(_) => return Err(RateLimitConfigError::FixedClockOutsideTest),
        };
        Ok(Self {
            limiter: RateLimiter::default(),
            trusted,
            production: *environment == Environment::Production,
            fixed_now,
        })
    }

    pub fn check(
        &self,
        peer: IpAddr,
        headers: &HeaderMap,
        method: &Method,
        path: &str,
    ) -> Option<Rejection> {
        if self.production && direct_loopback(peer, &self.trusted) {
            return None;
        }
        let now = self.fixed_now.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        self.limiter
            .check(client_ip(peer, headers, &self.trusted), method, path, now)
    }
}

fn direct_loopback(peer: IpAddr, trusted: &HashSet<IpAddr>) -> bool {
    peer.is_loopback() && !trusted.contains(&peer)
}

fn operation_rule(method: &Method, path: &str) -> Option<Rule> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if parts.len() < 5
        || parts[0] != "api"
        || parts[1] != "v1"
        || parts[2] != "households"
        || parts[3].is_empty()
    {
        return None;
    }
    match (method, parts[4], parts.get(5), parts.len()) {
        (&Method::GET | &Method::HEAD, "data_exports", Some(mode), 6) if !mode.is_empty() => {
            Some(Rule::DataExport)
        }
        (&Method::POST, "sync", Some(&"batches"), 6) => Some(Rule::SyncBatch),
        (&Method::GET | &Method::HEAD, "medication_lookup", None, 5) => {
            Some(Rule::MedicationLookup)
        }
        (&Method::POST, "ai_medication_suggestions", None, 5) => Some(Rule::AiSuggestion),
        (&Method::GET | &Method::HEAD, "admin", Some(&"audit_logs"), 6) => Some(Rule::AdminAudit),
        _ => None,
    }
}

fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &HashSet<IpAddr>) -> IpAddr {
    if !trusted.contains(&peer) {
        return peer;
    }
    let Some(forwarded) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    else {
        return peer;
    };
    let mut current = peer;
    for value in forwarded.split(',').rev() {
        if !trusted.contains(&current) {
            break;
        }
        let Ok(candidate) = value.trim().parse::<IpAddr>() else {
            return peer;
        };
        current = candidate;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::{RateLimitState, RateLimiter, Rule, client_ip, direct_loopback, operation_rule};
    use axum::http::{HeaderMap, HeaderValue, Method};
    use loco_rs::environment::Environment;
    use serde_json::json;
    use std::collections::HashSet;
    use std::net::IpAddr;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn fixed_windows_reject_the_301st_request_and_reclaim_expired_buckets() {
        let limiter = RateLimiter::default();
        let ip: IpAddr = "198.51.100.20".parse().unwrap();
        for _ in 0..300 {
            assert!(
                limiter
                    .check(ip, &Method::GET, "/api/v1/capabilities", 1_000)
                    .is_none()
            );
        }
        let rejection = limiter
            .check(ip, &Method::GET, "/api/v1/capabilities", 1_000)
            .unwrap();
        assert_eq!(
            (rejection.limit, rejection.retry_after, rejection.reset_at),
            (300, 200, 1_200)
        );
        assert_eq!(limiter.bucket_count(), 1);
        assert!(
            limiter
                .check(ip, &Method::GET, "/api/v1/capabilities", 1_200)
                .is_none()
        );
        assert_eq!(limiter.bucket_count(), 1);
    }

    #[test]
    fn operation_limits_and_methods_match_the_contract() {
        let limiter = RateLimiter::default();
        for (index, (method, path, limit)) in [
            (Method::GET, "/api/v1/households/42/data_exports/7", 10),
            (Method::POST, "/api/v1/households/42/sync/batches", 30),
            (Method::GET, "/api/v1/households/42/medication_lookup", 60),
            (
                Method::POST,
                "/api/v1/households/42/ai_medication_suggestions",
                10,
            ),
            (Method::GET, "/api/v1/households/42/admin/audit_logs", 100),
        ]
        .into_iter()
        .enumerate()
        {
            let ip: IpAddr = format!("198.51.100.{}", index + 21).parse().unwrap();
            for _ in 0..limit {
                assert!(limiter.check(ip, &method, path, 1_000).is_none());
            }
            assert_eq!(
                limiter.check(ip, &method, path, 1_000).unwrap().limit,
                limit
            );
        }
        let ip: IpAddr = "198.51.100.30".parse().unwrap();
        for _ in 0..11 {
            assert!(
                limiter
                    .check(
                        ip,
                        &Method::GET,
                        "/api/v1/households/42/data_exports",
                        1_000
                    )
                    .is_none()
            );
            assert!(
                limiter
                    .check(
                        ip,
                        &Method::POST,
                        "/api/v1/households/42/data_exports/7",
                        1_000
                    )
                    .is_none()
            );
        }
    }

    #[test]
    fn simultaneous_requests_increment_one_counter_atomically() {
        let limiter = Arc::new(RateLimiter::default());
        let barrier = Arc::new(Barrier::new(8));
        let ip: IpAddr = "198.51.100.22".parse().unwrap();
        let mut workers = Vec::new();
        for _ in 0..8 {
            let limiter = Arc::clone(&limiter);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                (0..50)
                    .filter(|_| {
                        limiter
                            .check(ip, &Method::GET, "/api/v1/capabilities", 1_000)
                            .is_none()
                    })
                    .count()
            }));
        }
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .sum::<usize>(),
            300
        );
    }

    #[test]
    fn head_and_get_share_read_operation_buckets() {
        let limiter = RateLimiter::default();
        for (index, (path, limit)) in [
            ("/api/v1/households/42/data_exports/health_data_json", 10),
            ("/api/v1/households/42/medication_lookup", 60),
            ("/api/v1/households/42/admin/audit_logs", 100),
        ]
        .into_iter()
        .enumerate()
        {
            let ip: IpAddr = format!("198.51.100.{}", index + 60).parse().unwrap();
            assert!(limiter.check(ip, &Method::HEAD, path, 1_000).is_none());
            for _ in 1..limit {
                assert!(limiter.check(ip, &Method::GET, path, 1_000).is_none());
            }
            assert_eq!(
                limiter.check(ip, &Method::HEAD, path, 1_000).unwrap().limit,
                limit
            );
        }
    }

    #[test]
    fn forwarding_headers_require_explicitly_trusted_peers() {
        let peer: IpAddr = "198.51.100.40".parse().unwrap();
        let client: IpAddr = "203.0.113.45".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.45"));
        assert_eq!(client_ip(peer, &headers, &HashSet::new()), peer);
        assert_eq!(client_ip(peer, &headers, &HashSet::from([peer])), client);

        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(direct_loopback(loopback, &HashSet::new()));
        let trusted = HashSet::from([loopback]);
        assert!(!direct_loopback(loopback, &trusted));
        assert_eq!(client_ip(loopback, &HeaderMap::new(), &trusted), loopback);
        headers.insert("x-forwarded-for", HeaderValue::from_static("malformed"));
        assert_eq!(client_ip(loopback, &headers, &trusted), loopback);
        headers.insert("x-forwarded-for", HeaderValue::from_static("127.0.0.1"));
        assert_eq!(client_ip(loopback, &headers, &trusted), loopback);
        assert_eq!(client_ip(peer, &headers, &HashSet::new()), peer);
        let limiter = RateLimiter::default();
        for _ in 0..300 {
            assert!(
                limiter
                    .check(loopback, &Method::GET, "/api/v1/capabilities", 1_000)
                    .is_none()
            );
        }
        assert!(
            limiter
                .check(loopback, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_some()
        );
    }

    #[test]
    fn trusted_hops_and_production_loopback_exemption_follow_socket_peer_policy() {
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        let proxy: IpAddr = "192.0.2.10".parse().unwrap();
        let client: IpAddr = "198.51.100.10".parse().unwrap();
        let trusted = HashSet::from([peer, proxy]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.10, 192.0.2.10"),
        );
        assert_eq!(client_ip(peer, &headers, &trusted), client);
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.10, invalid"),
        );
        assert_eq!(client_ip(peer, &headers, &trusted), peer);

        let direct = RateLimitState {
            limiter: RateLimiter::default(),
            trusted: HashSet::new(),
            production: true,
            fixed_now: Some(1_000),
        };
        for _ in 0..301 {
            assert!(
                direct
                    .check(
                        peer,
                        &headers,
                        &Method::GET,
                        "/api/v1/households/42/data_exports/7"
                    )
                    .is_none()
            );
        }
        let proxied = RateLimitState {
            limiter: RateLimiter::default(),
            trusted: HashSet::from([peer]),
            production: true,
            fixed_now: Some(1_000),
        };
        for _ in 0..10 {
            assert!(
                proxied
                    .check(
                        peer,
                        &HeaderMap::new(),
                        &Method::GET,
                        "/api/v1/households/42/data_exports/7"
                    )
                    .is_none()
            );
        }
        assert_eq!(
            proxied
                .check(
                    peer,
                    &HeaderMap::new(),
                    &Method::GET,
                    "/api/v1/households/42/data_exports/7"
                )
                .unwrap()
                .limit,
            10
        );
    }

    #[test]
    fn capacity_is_bounded_and_expired_entries_are_reclaimed() {
        let limiter = RateLimiter::with_capacity(2);
        let first: IpAddr = "198.51.100.31".parse().unwrap();
        let second: IpAddr = "198.51.100.32".parse().unwrap();
        let third: IpAddr = "198.51.100.33".parse().unwrap();
        assert!(
            limiter
                .check(first, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_none()
        );
        assert!(
            limiter
                .check(second, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_none()
        );
        assert!(
            limiter
                .check(third, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_some()
        );
        assert_eq!(limiter.bucket_count(), 2);
        assert!(
            limiter
                .check(third, &Method::GET, "/api/v1/capabilities", 1_200)
                .is_none()
        );
        assert_eq!(limiter.bucket_count(), 1);
    }

    #[test]
    fn special_limits_only_match_documented_paths_and_methods() {
        for (method, path) in [
            (Method::GET, "/api/v1/households/42/data_exports/7/extra"),
            (Method::GET, "/api/v1/households/42/admin/audit_logs/7"),
            (Method::POST, "/api/v1/households/42/data_exports/7"),
            (Method::POST, "/api/v1/households/42/admin/audit_logs"),
            (Method::GET, "/api/v1/households/42/sync/batches"),
        ] {
            assert_eq!(operation_rule(&method, path), None, "{method} {path}");
        }
        assert_eq!(
            operation_rule(&Method::GET, "/api/v1/households/42/data_exports/7"),
            Some(Rule::DataExport)
        );
        assert_eq!(
            operation_rule(&Method::GET, "/api/v1/households/42/admin/audit_logs"),
            Some(Rule::AdminAudit)
        );
    }

    #[test]
    fn trusted_proxy_configuration_rejects_empty_or_malformed_members() {
        for value in [
            "127.0.0.1,",
            ",127.0.0.1",
            "127.0.0.1,,198.51.100.1",
            "not-an-ip",
        ] {
            assert!(
                RateLimitState::from_settings(
                    Some(&json!({"api_rate_limit":{"trusted_proxy_ips":value}})),
                    &Environment::Test
                )
                .is_err(),
                "{value}"
            );
        }
        assert!(
            RateLimitState::from_settings(
                Some(&json!({"api_rate_limit":{"trusted_proxy_ips":""}})),
                &Environment::Test
            )
            .is_ok()
        );
        assert!(
            RateLimitState::from_settings(
                Some(&json!({"api_rate_limit":{"fixed_now_unix_seconds":1000}})),
                &Environment::Production
            )
            .is_err()
        );
    }
}
