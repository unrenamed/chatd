use std::time::Duration;

use governor::clock::{Clock, QuantaClock, Reference};
use governor::RateLimiter;

pub type RateLimit = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

pub fn check(rl: &RateLimit) -> Result<(), Duration> {
    let err = rl.check().err();

    match err {
        Some(nu) => {
            let now = QuantaClock::default().now();
            let remaining_nanos = nu.earliest_possible().duration_since(now);
            let remaining_duration = Duration::from_nanos(remaining_nanos.as_u64());
            let truncated_remaining_duration = Duration::new(remaining_duration.as_secs(), 0);
            Err(truncated_remaining_duration)
        }
        None => Ok(()),
    }
}
