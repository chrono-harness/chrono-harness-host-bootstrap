# chrono-harness-host-bootstrap

按配方准备宿主工具。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/host-bootstrap)

启动前先由宿主安装 chrono-judge-registration v0.1.0-alpha.1，并将其置于 PATH。初次构建、原始报告检查和证明构建都从固定或当前项目登记的原始字节调用 execution-methods；随后 bootstrap 选择 Rust 工具链并执行登记操作。

本仓发行在 Linux x86_64 和 macOS arm64 分别构建。安装或回退时从同一标签读取二进制和 SHA256SUMS，核对对应平台摘要。

启用生命周期所有者升级或原构建复用的宿主需要在 PATH 中安装提供 `adopted-owner-v1` 的 chrono-worktree。工作树程序核对属于它的部署状态。bootstrap 对返回的暂存记录和本仓构建报告、当前选中的可执行文件再做比对。普通首次构建以及独立的 Cargo-Verus 编译沿用各自登记的工具。

### 构建报告查询

在宿主根目录运行 `chrono-host-bootstrap original-report-v1`。从 stdin 读一个 JSON 对象，成功时 stdout 输出 JSON 并退出 0，失败时 stderr 说明原因并退出 2。

| operation | 输入 | 返回值 |
| --- | --- | --- |
| `checkout` | 宿主相对路径 `profile` | 当前 Git 源和平台、原报告的完整 UTF-8 文本，以及校验后的安装目标、安装摘要和输出流描述 |
| `retained` | `profile`、`platform`、`source`、原文 `report`、保存输出流的绝对路径 `directory` | 用登记和两流原字节校验后的安装及流摘要。可传 `revision: {"commit":"…","tree":"…"}` 从固定 Git 树读取登记，该修订必须与报告的源一致 |
| `streams` | 原文 `report` | 按操作顺序列出 stdout、stderr 的路径、长度和 SHA-256。此操作仅解码描述 |
| `source` | 无 | 当前宿主的 commit、tree、dirty 和观察错误 |
| `platform` | 无 | 运行平台的 system 和 machine |

bootstrap 独立构建。两个 Cargo 项目从本仓和 crates.io 解析源码。运行宿主构建时，PATH 中的 `chrono-harness` 负责准入编译器输出、检查保留的进程证据和管理登记产物。原生子进程继承真实的 Unix 文件描述符。即使 bootstrap 提前退出，仍在工作的子进程继续持有相关租约。配置了 `.chrono-harness/logging.json` 的宿主通过已安装的 `chrono-diagnostics cli-events-v1` 写入滚动日志。

从 GitHub 模板创建的宿主在仓库根目录运行 `chrono-host-bootstrap prepare-template`。程序从声明的 GitHub origin 识别仓库，通过匹配的 Actions 事件或远端 HEAD 取得默认分支，并更新 `.chrono-harness/template-answers.yml` 的两个身份字段。选装 `chrono-pr` 时，从本仓 `pr-policy.json` 生成 `pr.json`。选装 `chrono-instructions` 时，调用已安装的 `sync-rules`。成功打印当前仓库和分支并退出 0，输入或命令出错时从 stderr 输出原因并退出 2。
