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

#[cfg(test)]
mod should {
    use super::*;
    use governor::{Quota, RateLimiter};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn check_rate_limit_not_exceeded() {
        let quota = Quota::with_period(Duration::from_millis(1)).unwrap();
        let rate_limiter = RateLimiter::direct(quota);
        assert!(check(&rate_limiter).is_ok());
    }

    #[tokio::test]
    async fn check_rate_limit_exceeded() {
        let quota = Quota::with_period(Duration::from_millis(1)).unwrap();
        let rate_limiter = RateLimiter::direct(quota);

        // Consume the rate limit
        rate_limiter.check().unwrap();

        match check(&rate_limiter) {
            Ok(_) => panic!("Expected rate limit to be exceeded"),
            Err(duration) => assert!(duration.as_secs() <= 1),
        }

        // Wait for the rate limiter to reset
        sleep(Duration::from_millis(1)).await;

        // Perform a check again, expecting the rate limit to be reset
        assert!(check(&rate_limiter).is_ok());
    }
}
