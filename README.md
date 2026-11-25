# chrono-harness-host-bootstrap

按配方准备宿主工具。

在仓库根目录运行 `mise exec -- ./check.sh`。本仓的 mise 配置固定 Verus 版本，检查按变更验证工具绑定和 Cargo 模块及其消费者。

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

独立宿主可用 `chrono-host-bootstrap check-compiler-sources native <dep-info 主名> <声明源>...` 核对原生 Rust 编译依赖。它读取同名 `.d` 并检查每一条声明源。Verus 的 `check-compiler-sources proof <输出库> <声明源>...` 读取库附带的 `.d`，核对输出目标和全部声明源的实际路径、长度及 SHA-256。两种调用均通过宿主 PATH 中的 `chrono-harness compiler-output-v1` 解析原件，拒绝时报告 `E_VERUS_COVERAGE`。 同源判定从完整参数中取得模式、输出和原有序源列表。native 读取的标准 Path 比较、普通文件类型与 canonicalize 结果为 OS 接口：同一路径直接核声明源；路径不同则要求头部指向普通文件且解析后与所选 .d 相同。proof 按编译器原顺序核对全部声明源的路径、长度与摘要。

需要在全部编译调用结束后收取证明结果的宿主，为每个登记操作保留本次的独立报告和输出库，并运行 `chrono-host-bootstrap check-compiler-proof <Verus 版本> <commit> <原 stdout JSON> <输出库> <声明源>...`。程序要求库是非空普通文件，通过 runner 的 `proof-rows` 核对上报的版本、commit 和整根结果，再给 `dependency-matches` 核对目标、路径、字节数和 SHA-256。成功时返回原报告字节；缺原件或有一项不合格时报 `E_VERUS_COVERAGE`。宿主从该次专属输出目录分别收取每个必需操作。

需要把证明报告归到具体操作时，在仓库根目录创建本次专属的临时目录，依次运行：

```sh
chrono-host-bootstrap run-compiler-proof "$scratch" "$operation" "$verus" "$root"
chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "$verus" "$version" "$commit" "$root" "${additional_sources[@]}"
```

第一条命令使用宿主指定的 Verus，创建独占的操作目录并保存其进程原件、输出库和 dep-info 摘要。第二条命令按登记的操作、根、可执行文件、原始命令行与本轮原件逐项收取，成功时输出原报告字节；缺少或错配时返回 `E_VERUS_COVERAGE`。宿主负责该临时目录的写入隔离与回收，以及 Verus 的选择、校验和安装。判断依赖进程、编译器、文件系统与摘要算法的实际行为。

原生 Rust 检查通过已安装 runner 的 `compiler-output-v1 dependency-header` 读取 rustc 当时写下的目标主名。复用 Cargo 缓存时，目录别名和包含 `..` 的路径须实际解析到本次选中库的同一个普通 dep-info 文件；另一个目录中的硬链接和指向该文件的符号链接会被拒绝。

当普通构建与证明使用同一组包依赖时，在仓库内的 Cargo workspace 登记包、库及库类型的 `[[example]]` 目标，提交同一份 `Cargo.lock`。需要验证的包由宿主选择。例如已有 `identifiers`、`verifier` 两个包的宿主可运行以下命令。

```sh
chrono-host-bootstrap run-cargo-proofs "$scratch" verify-packages "$cargo_verus" crates/proofs/Cargo.toml identifiers verifier
chrono-host-bootstrap collect-cargo-proof "$scratch" verify-packages "$cargo_verus" crates/proofs/Cargo.toml verifier "$version" "$commit" crates/proofs/verifier/Cargo.toml crates/proofs/verifier/src/lib.rs
```

从宿主根目录调用。传入已存在的本次临时目录、Verus 分发中的 `cargo-verus` 以及 workspace 根 manifest。执行器用 Cargo metadata 获取所选包的目标，用该 workspace 的锁文件在新 target 目录串行编译。宿主为每个必需的根收取一次结果，需要绑定的其他源可接在根路径后。收取成功返回该根的原始 Verus JSON。调用、根、原始流、库、验证元数据或编译器记录的输入与本轮不符时，命令以退出码 1 报告 `E_VERUS_COVERAGE`。

`.d` 给出 rustc 记录的源和二进制依赖及其校验值。`.rmeta`、`.vir` 和 `.rlib` 均核对产生时的内容。库目标的 `.rlib` 在本步骤是构建产物；最终程序是否链接它需要在最终链接步骤查验。原始 stdout、stderr、收据与 Cargo target 均在调用者的临时目录中，收取结束后随该次工作清理。

## 本地检查的保留窗口

本仓的本地完整检查在 `.chrono-harness/state/collection/check.json` 发布报告。需要保留的结果应在 24 小时内将报告及其引用的原件转存至检出外，并从接收位置读回。各 Verus 操作的编译库和依赖文件由项目判官读取，成功后由当前操作释放。

运行 `mise run harness-maintain-checks` 可执行一轮本仓的检查产物维护。已到期的成功报告在读取和交付义务结束后释放，后续维护轮次回收失去引用的原件。没有业务事件或显式维护时，到期的文件会继续占用磁盘。

## 原生检查的临时编译目录

从宿主仓库根运行 `chrono-host-bootstrap native-scratch run PROGRAM [ARG ...]`。程序从环境变量 `CHRONO_NATIVE_SCRATCH` 读取专属的绝对目录，路径位于 `target/native-checks/`。成功退出时释放本次目录；失败时保留到宿主设定的窗口。标准错误在启动子程序前给出目录，需长期留存的数据请在窗口内转存。外部自选的 Cargo target 仍由调用方管理。

将 `chrono-host-bootstrap native-scratch maintain 86400` 登记为宿主的维护操作，可在最后写入或失败满 24 小时且最后一个持锁进程结束后清理。参数是正整数，以秒为单位。工具核对 Git 索引和 HEAD，保留跟踪路径，以及身份不符的目录。下一次成功处理该目录的维护轮次会回收符合条件的材料。
