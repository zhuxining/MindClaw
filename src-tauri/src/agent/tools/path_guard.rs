use crate::error::AppError;
use std::path::{Component, Path, PathBuf};

/// 路径访问策略：类 Tauri capability 模型
///
/// 默认授权 vault 根目录，可追加其他授权目录（如用户通过 dialog 授权的路径）。
/// 所有解析均做组件级边界检查，防止路径穿越。
pub struct PathGuard {
    primary: PathBuf,
    extra_roots: Vec<PathBuf>,
    denied_prefixes: Vec<String>,
}

impl PathGuard {
    /// 仅 vault 根目录可访问
    pub fn vault_only(vault_root: impl Into<PathBuf>) -> Self {
        Self {
            primary: vault_root.into(),
            extra_roots: Vec::new(),
            denied_prefixes: Vec::new(),
        }
    }

    /// 追加额外授权目录（如用户通过 dialog 选择的文件夹）
    pub fn with_extra(mut self, path: impl Into<PathBuf>) -> Self {
        self.extra_roots.push(path.into());
        self
    }

    /// 追加拒绝的子路径前缀（相对路径，如 "private"）
    pub fn with_denied(mut self, prefix: impl Into<String>) -> Self {
        self.denied_prefixes.push(prefix.into());
        self
    }

    /// 返回主工作区根目录
    pub fn primary(&self) -> &Path {
        &self.primary
    }

    /// 遍历所有授权根（primary + extra_roots）
    pub fn all_roots(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.primary.as_path()).chain(self.extra_roots.iter().map(PathBuf::as_path))
    }

    /// 解析并校验路径：
    /// - 相对路径 → primary.join(path)
    /// - 绝对路径 → 必须在某个授权根内
    /// - 规范化后做组件级 starts_with 比较（防路径穿越）
    /// - 路径中任何组件匹配 denied_prefixes → 拒绝
    pub fn resolve(&self, path: &str) -> Result<PathBuf, AppError> {
        let raw = if path.is_empty() {
            self.primary.clone()
        } else {
            let p = Path::new(path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                self.primary.join(path)
            }
        };

        let normalized = normalize_path(&raw);

        // 检查是否在某个授权根内
        let in_allowed = self
            .all_roots()
            .any(|root| normalized.starts_with(normalize_path(root)));

        if !in_allowed {
            return Err(AppError::PermissionDenied(format!(
                "path '{}' is outside allowed directories",
                path
            )));
        }

        // 检查 denied_prefixes（对规范化后的相对路径各组件进行匹配）
        for root in self.all_roots() {
            let norm_root = normalize_path(root);
            if let Ok(rel) = normalized.strip_prefix(&norm_root) {
                for component in rel.components() {
                    if let Component::Normal(name) = component {
                        let name_str = name.to_string_lossy();
                        for denied in &self.denied_prefixes {
                            if name_str == denied.as_str()
                                || name_str.starts_with(&format!("{}/", denied))
                            {
                                return Err(AppError::PermissionDenied(format!(
                                    "access to '{}' is denied: {}",
                                    denied, path
                                )));
                            }
                        }
                    }
                }
            }
        }

        Ok(normalized)
    }
}

/// 规范化路径：展开 `..` 和 `.` 组件，不访问文件系统
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => result.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(c) => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn guard(root: &str) -> PathGuard {
        PathGuard::vault_only(PathBuf::from(root)).with_denied("private")
    }

    #[test]
    fn test_resolve_relative() {
        let g = guard("/vault");
        let p = g.resolve("notes/foo.md").unwrap();
        assert_eq!(p, PathBuf::from("/vault/notes/foo.md"));
    }

    #[test]
    fn test_resolve_empty_is_root() {
        let g = guard("/vault");
        let p = g.resolve("").unwrap();
        assert_eq!(p, PathBuf::from("/vault"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let g = guard("/vault");
        assert!(g.resolve("../etc/passwd").is_err());
    }

    #[test]
    fn test_private_blocked() {
        let g = guard("/vault");
        assert!(g.resolve("private/secret.txt").is_err());
    }

    #[test]
    fn test_nested_private_blocked() {
        let g = guard("/vault");
        assert!(g.resolve("notes/private/secret.txt").is_err());
    }

    #[test]
    fn test_extra_root_allowed() {
        let g = guard("/vault").with_extra("/docs");
        let p = g.resolve("/docs/readme.md").unwrap();
        assert_eq!(p, PathBuf::from("/docs/readme.md"));
    }

    #[test]
    fn test_outside_all_roots_blocked() {
        let g = guard("/vault").with_extra("/docs");
        assert!(g.resolve("/home/user/secret.txt").is_err());
    }
}
