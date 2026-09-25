use axum::http::{HeaderMap, Method};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Mutex;

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
        if self == Self::General {
            300
        } else {
            60
        }
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
pub(super) struct Rejection {
    pub limit: u32,
    pub retry_after: u64,
    pub reset_at: u64,
}

pub(super) struct RateLimiter {
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

pub(super) fn direct_loopback(peer: IpAddr, trusted: &HashSet<IpAddr>) -> bool {
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
        (&Method::GET, "data_exports", Some(_), _) => Some(Rule::DataExport),
        (&Method::POST, "sync", Some(&"batches"), 6) => Some(Rule::SyncBatch),
        (&Method::GET, "medication_lookup", None, 5) => Some(Rule::MedicationLookup),
        (&Method::POST, "ai_medication_suggestions", None, 5) => Some(Rule::AiSuggestion),
        (&Method::GET, "admin", Some(&"audit_logs"), _) => Some(Rule::AdminAudit),
        _ => None,
    }
}

pub(super) fn trusted_proxy_ips() -> HashSet<IpAddr> {
    std::env::var("API_TRUSTED_PROXY_IPS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|value| value.trim().parse().ok())
        .collect()
}

pub(super) fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &HashSet<IpAddr>) -> IpAddr {
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
    use super::{client_ip, direct_loopback, RateLimiter};
    use axum::http::{HeaderMap, HeaderValue, Method};
    use std::collections::HashSet;
    use std::net::IpAddr;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn fixed_windows_reject_the_301st_request_and_reclaim_expired_buckets() {
        let limiter = RateLimiter::default();
        let ip: IpAddr = "198.51.100.20".parse().unwrap();
        for _ in 0..300 {
            assert!(limiter
                .check(ip, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_none());
        }
        let rejection = limiter
            .check(ip, &Method::GET, "/api/v1/capabilities", 1_000)
            .unwrap();
        assert_eq!(
            (rejection.limit, rejection.retry_after, rejection.reset_at),
            (300, 200, 1_200)
        );
        assert_eq!(limiter.bucket_count(), 1);
        assert!(limiter
            .check(ip, &Method::GET, "/api/v1/capabilities", 1_200)
            .is_none());
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
            (Method::GET, "/api/v1/households/42/admin/audit_logs/7", 100),
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
            assert!(limiter
                .check(
                    ip,
                    &Method::GET,
                    "/api/v1/households/42/data_exports",
                    1_000
                )
                .is_none());
            assert!(limiter
                .check(
                    ip,
                    &Method::POST,
                    "/api/v1/households/42/data_exports/7",
                    1_000
                )
                .is_none());
        }
    }

    #[test]
    fn simultaneous_requests_increment_one_counter_atomically() {
        let limiter = Arc::new(RateLimiter::default());
        let ip: IpAddr = "198.51.100.22".parse().unwrap();
        let mut workers = Vec::new();
        for _ in 0..8 {
            let limiter = Arc::clone(&limiter);
            workers.push(thread::spawn(move || {
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
            assert!(limiter
                .check(loopback, &Method::GET, "/api/v1/capabilities", 1_000)
                .is_none());
        }
        assert!(limiter
            .check(loopback, &Method::GET, "/api/v1/capabilities", 1_000)
            .is_some());
    }

    #[test]
    fn capacity_is_bounded_and_expired_entries_are_reclaimed() {
        let limiter = RateLimiter::with_capacity(2);
        let first: IpAddr = "198.51.100.31".parse().unwrap();
        let second: IpAddr = "198.51.100.32".parse().unwrap();
        let third: IpAddr = "198.51.100.33".parse().unwrap();
        assert!(limiter
            .check(first, &Method::GET, "/api/v1/capabilities", 1_000)
            .is_none());
        assert!(limiter
            .check(second, &Method::GET, "/api/v1/capabilities", 1_000)
            .is_none());
        assert!(limiter
            .check(third, &Method::GET, "/api/v1/capabilities", 1_000)
            .is_some());
        assert_eq!(limiter.bucket_count(), 2);
        assert!(limiter
            .check(third, &Method::GET, "/api/v1/capabilities", 1_200)
            .is_none());
        assert_eq!(limiter.bucket_count(), 1);
    }
}
