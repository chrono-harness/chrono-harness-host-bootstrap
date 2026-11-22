# chrono-harness-host-bootstrap

按配方准备宿主工具。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-bootstrap)

启动前先由宿主安装 chrono-judge-registration v0.1.0-alpha.1，并将其置于 PATH。初次构建、原始报告检查和证明构建都从固定或当前项目登记的原始字节调用 execution-methods；随后 bootstrap 选择 Rust 工具链并执行登记操作。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。
