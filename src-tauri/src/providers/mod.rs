pub mod claude;
pub mod config;
pub mod traits;

use crate::error::AppResult;
use claude::ClaudeProvider;
use traits::ModelTier;

/// 根据配置创建对应的 Provider 实例
pub fn create_provider(_api_key: String, _tier: ModelTier) -> AppResult<ClaudeProvider> {
    todo!("实现 Provider 工厂")
}
