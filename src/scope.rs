//! 轻量 scope:把 `.colna/scopes.json` 里的名字映射到 memory/ 路径前缀。
//!
//! 多 vault 仍用 `--root` 选择不同库;scope 用于同一库内工作/个人隔离。

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
struct ScopeConfig {
    #[serde(default)]
    scopes: BTreeMap<String, String>,
}

pub fn resolve(root: &Path, scope: Option<&str>) -> Result<Option<String>> {
    let Some(scope) = scope.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if scope.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    let synced = root.join("memory/.colna-scopes.json");
    let local = root.join(".colna/scopes.json");
    let path = if synced.exists() { synced } else { local };
    if !path.exists() {
        bail!(
            "未配置 scope:{}。请创建 `memory/.colna-scopes.json`(随库同步)或 `.colna/scopes.json`(本机),例如 {{\"scopes\":{{\"work\":\"10-Projects/work\"}}}}",
            path.display()
        );
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("读取 scope 配置失败: {}", path.display()))?;
    let config: ScopeConfig =
        serde_json::from_str(&content).with_context(|| "解析 `.colna/scopes.json` 失败")?;
    match config.scopes.get(scope) {
        Some(prefix) => Ok(Some(prefix.trim_matches('/').to_string())),
        None => bail!(
            "未知 scope: {scope};可用: {:?}",
            config.scopes.keys().collect::<Vec<_>>()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_configured_scope() {
        let root = std::env::temp_dir().join(format!(
            "colna-scope-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join(".colna")).unwrap();
        std::fs::write(
            root.join(".colna/scopes.json"),
            r#"{"scopes":{"work":"10-Projects/work"}}"#,
        )
        .unwrap();
        assert_eq!(
            resolve(&root, Some("work")).unwrap().as_deref(),
            Some("10-Projects/work")
        );
        assert!(resolve(&root, Some("personal")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
