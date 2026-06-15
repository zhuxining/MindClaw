use std::time::Duration;

/// 重试策略
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 500,
        }
    }
}

impl RetryPolicy {
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        Duration::from_millis(self.base_delay_ms * 2u64.saturating_pow(attempt.saturating_sub(1)))
    }
}
