//! 附件 / 文档摄取:PDF、HTML、图片 OCR、音视频转写 → Markdown。
//!
//! 外部工具通过 PATH 或 `COLNA_WHISPER_CMD` 提供;缺失时给出明确错误,
//! 不把二进制内容直接塞进索引。

use crate::indexer;
use crate::state;
use crate::util;
use crate::write;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn ingest(root: &Path, path: &Path, title: Option<&str>, tags: &str) -> Result<PathBuf> {
    let (kind, text) = extract(path)?;
    if text.trim().is_empty() {
        bail!("没有抽取到文本: {}", path.display());
    }
    let title = title
        .map(str::to_string)
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Ingested".to_string());
    let source = source_label(path);
    let extracted_at = util::now_beijing();
    let content_hash = state::content_hash(&text);
    let extra = [
        ("source_path", source.as_str()),
        ("source_type", kind),
        ("extracted_at", extracted_at.as_str()),
        ("content_hash", content_hash.as_str()),
    ];
    write::add_note(root, &title, tags, &text, &extra, |root| {
        indexer::ensure(root, false, false, None).map(|_| ())
    })
}

fn source_label(path: &Path) -> String {
    if path.is_relative() {
        return path.to_string_lossy().to_string();
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(relative) = path.strip_prefix(&cwd) {
            return relative.to_string_lossy().to_string();
        }
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment")
        .to_string()
}

fn extract(path: &Path) -> Result<(&'static str, String)> {
    if !path.is_file() {
        bail!("文件不存在: {}", path.display());
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "md" | "markdown" | "txt" => Ok(("text", std::fs::read_to_string(path)?)),
        "pdf" => Ok((
            "pdf",
            run_tool("pdftotext", &["-layout", path_str(path)?, "-"])?,
        )),
        "html" | "htm" => {
            let text = run_tool("textutil", &["-convert", "txt", "-stdout", path_str(path)?])
                .or_else(|_| run_tool("pandoc", &["-f", "html", "-t", "plain", path_str(path)?]))?;
            Ok(("html", text))
        }
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "webp" => {
            let text = run_tool(
                "tesseract",
                &[path_str(path)?, "stdout", "-l", "chi_sim+eng"],
            )
            .or_else(|_| run_tool("tesseract", &[path_str(path)?, "stdout", "-l", "eng"]))?;
            Ok(("image", text))
        }
        "mp3" | "m4a" | "wav" | "flac" | "aac" | "mp4" | "mov" | "mkv" => {
            let command = std::env::var("COLNA_WHISPER_CMD").map_err(|_| {
                anyhow::anyhow!(
                    "音视频转写需要设置 COLNA_WHISPER_CMD(命令收到文件路径作为 $1,输出转写文本)"
                )
            })?;
            let output = Command::new("sh")
                .args(["-c", &format!("{command} \"$1\""), "colna-whisper"])
                .arg(path)
                .output()
                .context("执行 COLNA_WHISPER_CMD 失败")?;
            if !output.status.success() {
                bail!(
                    "转写命令失败: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
            Ok(("audio", String::from_utf8_lossy(&output.stdout).to_string()))
        }
        other => bail!("不支持的附件类型: {other}"),
    }
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("路径包含非 UTF-8: {}", path.display()))
}

fn run_tool(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("执行 {command} 失败(是否安装?)"))?;
    if !output.status.success() {
        bail!(
            "{command} 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_source_label_for_relative_paths() {
        let label = source_label(Path::new("docs/example.pdf"));
        assert_eq!(label, "docs/example.pdf");
    }

    #[test]
    fn rejects_missing_files() {
        let error = extract(Path::new("/definitely/missing/file.pdf")).unwrap_err();
        assert!(error.to_string().contains("文件不存在"));
    }
}
