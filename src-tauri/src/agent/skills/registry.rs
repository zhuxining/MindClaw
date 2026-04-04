//! SkillsRegistry - 技能注册表
//!
//! 渐进式披露：
//! - 启动阶段：扫描所有 SKILL.md，只解析 frontmatter（~100 tokens）
//! - 激活阶段：根据用户消息匹配技能，加载完整 SKILL.md（< 5000 tokens）
//! - 按需加载：scripts/、references/、assets/ 资源文件

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

// ============================================================================
// 核心类型
// ============================================================================

/// 技能清单（完整）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// 技能名称（1-64字符，小写/数字/连字符，匹配目录名）
    pub name: String,
    /// 描述（1-1024字符，说明做什么和何时使用）
    pub description: String,
    /// 许可证
    pub license: Option<String>,
    /// 环境要求
    pub compatibility: Option<String>,
    /// 扩展字段
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 预批准工具列表
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 是否常驻加载
    #[serde(default)]
    pub always_load: bool,
    /// Markdown body 内容
    #[serde(skip)]
    pub content: String,
    /// 来源路径
    pub path: PathBuf,
}

/// 轻量级元数据（启动时加载）
#[derive(Debug, Clone, Serialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub always_load: bool,
    pub path: PathBuf,
}

impl From<&SkillManifest> for SkillMetadata {
    fn from(manifest: &SkillManifest) -> Self {
        Self {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            always_load: manifest.always_load,
            path: manifest.path.clone(),
        }
    }
}

// ============================================================================
// SkillsRegistry
// ============================================================================

/// 技能注册表
pub struct SkillsRegistry {
    /// 搜索路径列表
    search_paths: Vec<PathBuf>,
    /// 轻量级元数据索引（启动时构建）
    metadata_index: HashMap<String, SkillMetadata>,
    /// 已激活的完整清单（按需缓存）
    manifests: RwLock<HashMap<String, SkillManifest>>,
}

impl SkillsRegistry {
    /// 创建新的技能注册表
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            metadata_index: HashMap::new(),
            manifests: RwLock::new(HashMap::new()),
        }
    }

    /// 扫描并索引所有技能（启动时调用）
    pub async fn scan(&mut self) -> Result<(), AppError> {
        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            let mut entries = tokio::fs::read_dir(search_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let skill_file = path.join("SKILL.md");
                if !skill_file.exists() {
                    continue;
                }

                // 只解析 frontmatter
                match self.parse_metadata(&skill_file).await {
                    Ok(metadata) => {
                        // 验证 name 与目录名匹配
                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        if metadata.name != dir_name {
                            tracing::warn!(
                                skill_name = %metadata.name,
                                dir_name = %dir_name,
                                "skill_name_mismatch"
                            );
                        }

                        self.metadata_index.insert(metadata.name.clone(), metadata);
                    }
                    Err(e) => {
                        tracing::warn!(path = %skill_file.display(), error = %e, "skill_parse_failed");
                    }
                }
            }
        }

        tracing::info!(skill_count = %self.metadata_index.len(), "skills_indexed");
        Ok(())
    }

    /// 解析技能元数据（仅 frontmatter）
    async fn parse_metadata(&self, path: &PathBuf) -> Result<SkillMetadata, AppError> {
        let content = tokio::fs::read_to_string(path).await?;

        // 解析 YAML frontmatter
        let (frontmatter, _) = parse_frontmatter(&content)?;

        let name = frontmatter
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("skill name required".to_string()))?
            .to_string();

        let description = frontmatter
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let always_load = frontmatter
            .get("metadata")
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("always_load"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(SkillMetadata {
            name,
            description,
            always_load,
            path: path.parent().unwrap().to_path_buf(),
        })
    }

    /// 激活技能（加载完整 SKILL.md）
    pub async fn activate(&self, name: &str) -> Result<SkillManifest, AppError> {
        // 检查缓存
        {
            let manifests = self.manifests.read().await;
            if let Some(manifest) = manifests.get(name) {
                return Ok(manifest.clone());
            }
        }

        // 从元数据索引获取路径
        let metadata = self
            .metadata_index
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("skill not found: {}", name)))?;

        // 加载完整清单
        let skill_file = metadata.path.join("SKILL.md");
        let manifest = self.parse_manifest(&skill_file).await?;

        // 缓存
        self.manifests
            .write()
            .await
            .insert(name.to_string(), manifest.clone());

        Ok(manifest)
    }

    /// 解析完整技能清单
    async fn parse_manifest(&self, path: &PathBuf) -> Result<SkillManifest, AppError> {
        let content = tokio::fs::read_to_string(path).await?;

        let (frontmatter, body) = parse_frontmatter(&content)?;

        let name = frontmatter
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = frontmatter
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let metadata: HashMap<String, String> = frontmatter
            .get("metadata")
            .and_then(|m| m.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let always_load = metadata
            .get("always_load")
            .map(|s| s == "true")
            .unwrap_or(false);

        let allowed_tools = frontmatter
            .get("allowed-tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(SkillManifest {
            name,
            description,
            license: frontmatter
                .get("license")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            compatibility: frontmatter
                .get("compatibility")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            metadata,
            allowed_tools,
            always_load,
            content: body.to_string(),
            path: path.parent().unwrap().to_path_buf(),
        })
    }

    /// 搜索技能（基于描述匹配）
    pub fn search(&self, query: &str) -> Vec<&SkillMetadata> {
        let query_lower = query.to_lowercase();
        self.metadata_index
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// 获取常驻技能（always_load = true）
    pub fn get_resident_skills(&self) -> Vec<&SkillMetadata> {
        self.metadata_index
            .values()
            .filter(|m| m.always_load)
            .collect()
    }

    /// 获取可发现技能（always_load = false）
    pub fn get_discoverable_skills(&self) -> Vec<&SkillMetadata> {
        self.metadata_index
            .values()
            .filter(|m| !m.always_load)
            .collect()
    }

    /// 获取所有技能元数据
    pub fn list(&self) -> Vec<&SkillMetadata> {
        self.metadata_index.values().collect()
    }

    /// 加载资源文件
    pub async fn load_resource(
        &self,
        skill: &str,
        relative_path: &str,
    ) -> Result<String, AppError> {
        let metadata = self
            .metadata_index
            .get(skill)
            .ok_or_else(|| AppError::NotFound(format!("skill not found: {}", skill)))?;

        let resource_path = metadata.path.join(relative_path);
        let content = tokio::fs::read_to_string(&resource_path).await?;

        Ok(content)
    }

    /// 技能数量
    pub fn len(&self) -> usize {
        self.metadata_index.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.metadata_index.is_empty()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 解析 YAML frontmatter（简化实现，不依赖 serde_yaml）
fn parse_frontmatter(
    content: &str,
) -> Result<(HashMap<String, serde_json::Value>, &str), AppError> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Ok((HashMap::new(), content));
    }

    // 找到结束的 ---
    let rest = &content[3..];
    let end_pos = rest.find("---");

    if let Some(pos) = end_pos {
        let frontmatter_str = &rest[..pos];
        let body = &rest[pos + 3..];

        // 简单解析 YAML frontmatter
        let mut frontmatter: HashMap<String, serde_json::Value> = HashMap::new();
        let mut current_key: Option<String> = None;
        let mut in_nested = false;

        for line in frontmatter_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // 检测嵌套对象开始
            if line.ends_with(':') && !line.starts_with(' ') && !line.starts_with('\t') {
                current_key = Some(line[..line.len() - 1].to_string());
                in_nested = true;
                frontmatter.insert(
                    current_key.clone().unwrap(),
                    serde_json::Value::Object(serde_json::Map::new()),
                );
                continue;
            }

            // 解析键值对
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();

                // 处理嵌套对象内的属性
                if in_nested && (line.starts_with(' ') || line.starts_with('\t')) {
                    if let Some(parent_key) = &current_key {
                        if let Some(serde_json::Value::Object(obj)) =
                            frontmatter.get_mut(parent_key)
                        {
                            let clean_key = key.trim_start();
                            let json_value = parse_yaml_value(value);
                            obj.insert(clean_key.to_string(), json_value);
                        }
                    }
                    continue;
                }

                // 顶层属性
                in_nested = false;
                current_key = None;
                frontmatter.insert(key.to_string(), parse_yaml_value(value));
            }
        }

        Ok((frontmatter, body.trim()))
    } else {
        Ok((HashMap::new(), content))
    }
}

/// 解析 YAML 值为 JSON 值
fn parse_yaml_value(value: &str) -> serde_json::Value {
    let value = value.trim();

    // 去除引号
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return serde_json::Value::String(value[1..value.len() - 1].to_string());
    }

    // 布尔值
    if value == "true" {
        return serde_json::Value::Bool(true);
    }
    if value == "false" {
        return serde_json::Value::Bool(false);
    }

    // 数字
    if let Ok(n) = value.parse::<i64>() {
        return serde_json::Value::Number(n.into());
    }
    if let Ok(n) = value.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return serde_json::Value::Number(num);
        }
    }

    // 数组（简化处理）
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        let items: Vec<serde_json::Value> = inner
            .split(',')
            .map(|s| serde_json::Value::String(s.trim().trim_matches('"').to_string()))
            .collect();
        return serde_json::Value::Array(items);
    }

    // 默认为字符串
    serde_json::Value::String(value.to_string())
}
