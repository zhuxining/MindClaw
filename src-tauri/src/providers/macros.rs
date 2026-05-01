//! 供应商定义辅助宏
//!
//! 简化 match 语句，避免硬编码分散。

/// 在 runner.rs 中使用的 match 宏
///
/// 替代硬编码的 match 语句，统一调用 run_with_model
#[macro_export]
macro_rules! match_completion_model {
    ($model:expr, $run_fn:ident, $($args:expr),*) => {
        match $model {
            $crate::providers::LLMCompletionModel::Anthropic(model) => {
                $run_fn(model, $($args),*).await
            }
            $crate::providers::LLMCompletionModel::OpenAI(model) => {
                $run_fn(model, $($args),*).await
            }
            $crate::providers::LLMCompletionModel::DeepSeek(model) => {
                $run_fn(model, $($args),*).await
            }
        }
    };
}
