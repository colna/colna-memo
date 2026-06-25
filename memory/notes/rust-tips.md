---
title: Rust 错误处理小记
date: 2026-06-25
tags: rust, 编程
---

# Result 与 anyhow

在应用层用 anyhow 统一错误类型,库层用 thiserror 定义具体错误。
`?` 运算符会自动做 From 转换,配合 `context` 给错误加上下文。

# 生命周期

借用检查器在编译期保证没有悬垂引用,闭包捕获要注意 move 语义。
