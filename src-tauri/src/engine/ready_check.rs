use std::time::{Duration, Instant};
use xyra_core::config::Config;

/// Schedules one automatic accept per found match, after the delay the player chose.
#[derive(Default)]
pub struct ReadyCheckAcceptor {
    pub accept_at: Option<Instant>,
    seen: bool,
}

impl ReadyCheckAcceptor {
    pub fn follow(&mut self, in_ready_check: bool, config: &Config) {
        if !in_ready_check {
            *self = ReadyCheckAcceptor::default();
        } else if !self.seen {
            self.seen = true;
            self.accept_at = config.auto_accept.then(|| Instant::now() + Duration::from_secs(config.accept_delay_seconds.into()));
        }
    }

    pub fn take_due(&mut self, now: Instant) -> bool {
        let due = self.accept_at.is_some_and(|at| at <= now);
        if due {
            self.accept_at = None;
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_each_found_match_once_after_the_delay() {
        let config = Config { auto_accept: true, accept_delay_seconds: 2, ..Config::default() };
        let mut acceptor = ReadyCheckAcceptor::default();
        acceptor.follow(true, &config);
        let at = acceptor.accept_at.unwrap();
        assert!(!acceptor.take_due(at - Duration::from_millis(1)));
        assert!(acceptor.take_due(at));
        acceptor.follow(true, &config);
        assert!(acceptor.accept_at.is_none());
        acceptor.follow(false, &config);
        acceptor.follow(true, &config);
        assert!(acceptor.accept_at.is_some());
        acceptor.follow(false, &Config::default());
        acceptor.follow(true, &Config::default());
        assert!(acceptor.accept_at.is_none());
    }
}
