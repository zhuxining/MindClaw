//! Retry Policy — LLM 调用重试机制
//!
//! 参考 nanobot 的 chat_with_retry 实现
//! 处理 transient error（429, 500, timeout 等）自动重试

use std::time::Duration;

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_attempts: usize,
    /// 基础延迟序列（指数退避）
    pub base_delays: Vec<Duration>,
    /// 最大延迟上限
    pub max_delay: Duration,
    /// 瞬态错误标识符（匹配这些字符串则重试）
    pub transient_markers: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delays: vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
            ],
            max_delay: Duration::from_secs(60),
            transient_markers: vec![
                "429".to_string(),
                "rate limit".to_string(),
                "500".to_string(),
                "502".to_string(),
                "503".to_string(),
                "504".to_string(),
                "overloaded".to_string(),
                "timeout".to_string(),
                "timed out".to_string(),
                "connection".to_string(),
                "server error".to_string(),
                "temporarily unavailable".to_string(),
            ],
        }
    }
}

impl RetryPolicy {
    /// 创建标准重试策略
    pub fn standard() -> Self {
        Self::default()
    }

    /// 创建持久重试策略（无限重试，直到成功或非瞬态错误）
    pub fn persistent() -> Self {
        Self {
            max_attempts: usize::MAX,
            base_delays: vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
            ],
            max_delay: Duration::from_secs(60),
            transient_markers: Self::default().transient_markers,
        }
    }

    /// 创建禁用重试策略
    pub fn disabled() -> Self {
        Self {
            max_attempts: 0,
            base_delays: vec![],
            max_delay: Duration::ZERO,
            transient_markers: vec![],
        }
    }

    /// 判断错误是否为瞬态（可重试）
    pub fn is_transient(&self, error: &str) -> bool {
        let lower = error.to_lowercase();
        self.transient_markers
            .iter()
            .any(|marker| lower.contains(marker))
    }

    /// 获取第 N 次重试的延迟时间
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let base = self
            .base_delays
            .get(attempt.min(self.base_delays.len() - 1))
            .copied()
            .unwrap_or(self.max_delay);
        base.min(self.max_delay)
    }
}

/// 重试模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetryMode {
    /// 标准模式：最多 3 次重试
    #[default]
    Standard,
    /// 持久模式：无限重试瞬态错误
    Persistent,
    /// 禁用重试
    Disabled,
}

impl RetryMode {
    pub fn to_policy(&self) -> RetryPolicy {
        match self {
            RetryMode::Standard => RetryPolicy::standard(),
            RetryMode::Persistent => RetryPolicy::persistent(),
            RetryMode::Disabled => RetryPolicy::disabled(),
        }
    }
}

/// 从错误内容提取 Retry-After 时间
pub fn extract_retry_after(content: &str) -> Option<Duration> {
    let lower = content.to_lowercase();

    // 简化实现：直接查找数字和单位
    // 支持 "retry after Xs", "try again in Xms", "wait Xm before retry"
    let keywords = ["retry after", "try again in", "wait"];

    for keyword in keywords {
        if let Some(pos) = lower.find(keyword) {
            // 从关键词后面找数字
            let after_keyword = &lower[pos + keyword.len()..];
            let trimmed = after_keyword.trim_start();

            // 收集数字
            let mut num_str = String::new();
            let mut chars = trimmed.chars().peekable();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() || ch == '.' {
                    num_str.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }

            if num_str.is_empty() {
                continue;
            }

            // 跳过空格，收集单位
            while let Some(&ch) = chars.peek() {
                if ch == ' ' {
                    chars.next();
                } else {
                    break;
                }
            }

            let mut unit_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_alphabetic() {
                    unit_str.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }

            if unit_str.is_empty() {
                // 默认秒
                unit_str = "s".to_string();
            }

            let value: f64 = num_str.parse().ok()?;
            return Some(to_retry_seconds(value, &unit_str));
        }
    }

    None
}

fn to_retry_seconds(value: f64, unit: &str) -> Duration {
    let unit_lower = unit.to_lowercase();
    if unit_lower.starts_with("ms") {
        Duration::from_millis((value / 1000.0 * 1000.0) as u64)
    } else if unit_lower.starts_with("m") && !unit_lower.starts_with("ms") {
        Duration::from_secs((value * 60.0) as u64)
    } else {
        Duration::from_secs(value as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_transient() {
        let policy = RetryPolicy::standard();

        assert!(policy.is_transient("Error: 429 rate limit exceeded"));
        assert!(policy.is_transient("Error: 500 internal server error"));
        assert!(policy.is_transient("Error: connection timeout"));
        assert!(policy.is_transient("Error: server overloaded"));

        assert!(!policy.is_transient("Error: invalid API key"));
        assert!(!policy.is_transient("Error: model not found"));
    }

    #[test]
    fn test_delay_for_attempt() {
        let policy = RetryPolicy::standard();

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        // 超出范围使用最后一个
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(4));
    }

    #[test]
    fn test_extract_retry_after() {
        assert_eq!(
            extract_retry_after("retry after 30s"),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            extract_retry_after("try again in 500ms"),
            Some(Duration::from_millis(500))
        );
        assert_eq!(
            extract_retry_after("wait 2 minutes before retry"),
            Some(Duration::from_secs(120))
        );
    }
}
