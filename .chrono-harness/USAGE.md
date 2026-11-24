# chrono-harness-host-bootstrap 使用说明

组件的用途、开发仓库和示例见 [Chrono Harness 用户与开发者入口](https://github.com/chrono-harness/chrono-harness-main)。

按配方准备宿主工具。

开发分支为 `dev`。日常 PR 合入开发分支，正式版从 `main` 发布。

首次检出运行 `mise trust` 和 `mise run harness-setup`。工具版本见 [.config/mise/conf.d/chrono-harness.toml](../.config/mise/conf.d/chrono-harness.toml)。

开始任务时运行 `mise run harness-setup` 刷新根指南，成功后重新阅读本仓的 `AGENTS.md` 或 `CLAUDE.md`。完整规则和本仓开发步骤见根 [CLAUDE.md](../CLAUDE.md)。

用 `mise exec -- chrono-pr pr-reminders` 查看本地提醒，再运行 `mise exec -- chrono-pr pr-status --pr NUMBER --head-sha SHA` 查询 GitHub。

## 本仓验证

产品位于 [`crates/host-bootstrap`](../crates/host-bootstrap)。原生测试位于 [`crates/host-bootstrap-tests`](../crates/host-bootstrap-tests)。在仓库根目录运行 `mise exec -- ./check.sh` 按 Git 变更选择工具绑定和 Cargo 模块的生产证明、独立边界预期和候选 bootstrap 自己的编译器启动与收集检查。GitHub 汇总读取固定候选的单元原件。
