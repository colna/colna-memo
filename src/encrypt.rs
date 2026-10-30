//! age 加密包装:敏感笔记加密后不应再进入索引。
//!
//! 需要本机安装 `age`;`colna encrypt` 默认输出 `<path>.age`,
//! 建议把明文路径加入 `.colnaignore` 或移到库外。

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn encrypt(
    path: &Path,
    recipient: Option<&str>,
    passphrase: bool,
    output: Option<&Path>,
) -> Result<PathBuf> {
    if !path.is_file() {
        bail!("文件不存在: {}", path.display());
    }
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(format!("{}.age", path.display())));

    let mut command = Command::new("age");
    if let Some(recipient) = recipient {
        command.args(["-r", recipient]);
    } else if passphrase || recipient.is_none() {
        command.arg("-p");
    }
    command.arg("-o").arg(&output).arg(path);

    let status = command
        .status()
        .context("执行 age 失败(是否安装 age? macOS: brew install age)")?;
    if !status.success() {
        bail!("age 加密失败");
    }
    Ok(output)
}

pub fn decrypt(path: &Path) -> Result<String> {
    if !path.is_file() {
        bail!("文件不存在: {}", path.display());
    }
    let output = Command::new("age")
        .args(["-d"])
        .arg(path)
        .output()
        .context("执行 age 失败(是否安装 age? macOS: brew install age)")?;
    if !output.status.success() {
        bail!(
            "age 解密失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
