# SpecGuard — 规格守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**面向 AI 原生研发的确定性需求、规格与验收治理。**

> **本地实现，信任能力有限。** 当前分支已有 Rust 源码、可运行的只读 CLI、schema 和测试；独立验收情况见[任务清单](openspec/changes/add-specification-baseline-analysis/tasks.md)及[实施证据](docs/implementation-progress.md)。历史 `main` 提交 `01137804aa465c1b931c73c9d211ff0d29b46c7a` 仅有设计文档，该清单不描述当前实施分支。生产审批认证、托管门禁和正式分发仍未完成。

## 为什么需要 SpecGuard

AI 可以快速生成需求和任务，却不能证明需求范围被正确理解。SpecGuard 计划检查稳定身份、追踪关系以及批准验收义务的变更。业务语义歧义仍需人工审查，不能把自然语言模型评分当成强制门禁证据。

~~~text
原始诉求 / OpenSpec / Spec Kit / Superpowers / 显式 Markdown
                              ↓
                     只读适配器 + 声明覆盖范围
                              ↓
              需求 → 验收 → 设计/任务 → 测试义务
                              ↓
                    结构校验 + 批准基线差异
                              ↓
                 领域发现 → GuardFacts → GuardEngine
                              ↓
               限定范围的决策与证据 → 可信 CI / FlowGuard
~~~

## 范围与典型场景

- 在实施前发现重复需求 ID、断开的引用和缺失的强制验收关系。
- 将候选与不可变批准基线比较，检测验收义务删除或结构化条件弱化。
- 识别规格变更影响的架构、任务和测试义务，使旧证据失效。
- 明确解释已分析范围、无法读取的来源以及需要人工判断的问题。

输入为显式选择的规格源、候选快照、受保护策略，以及需要时的基线和经外部身份系统核验的审批记录。当前本地输出包括规格图、精确定位的发现、基线差异、覆盖清单以及兼容引擎的事实/报告引用。测试义务表示需要获得什么证据，不代表行为测试已经通过。

SpecGuard 负责规格解析和领域检查；ArchGuard 负责架构检查；CodeGuard 负责代码检查；TestGuard 负责测试证据；GitGuard 负责 Git 检查；FlowGuard 编排门禁及可信审批核验。六个守卫独立使用 [GuardEngine](https://github.com/full-stack-plugins/guardengine)，由引擎负责通用合同校验、中立规则评估和确定性证据计算。SpecGuard 和引擎都不授予业务批准、合并或发布权限。

## 决策、覆盖与兼容

确定性违例可以映射为 `enforce`；语义不确定性采用 `review`；建议采用 `advise`。引擎决策为 `ALLOW`、`BLOCK`、`REQUIRE_APPROVAL`。partial 事实意味着 `BLOCK` 和 `INDETERMINATE` 评估，不能表示完整检查通过。“完整”仅指声明的分析器范围，不代表理解了全部业务。人工批准不能覆盖分析失败或覆盖不足。

共享的当前 `guard.partme.ai/v1alpha1` 协议仅支持 GuardContract YAML、GuardFacts JSON、GuardReport JSON 和精确 `forbid_relation` 断言，拒绝未知字段。图集合量化检查应放在SpecGuard 领域验证器中；来源位置、审批和编排信息不能私自添加为引擎字段。报告无签名；verify 只重算结果，不证明信任或授权。本地生产者已生成并核验真实 GE 工件，见[冻结兼容能力](docs/frozen-integration-capabilities.md)；可复算不等于生产者认证。

[集成合同](docs/integration-contract.md)使用独立的 `GuardRunEnvelope`（`guard.integration/v1alpha1`），本地适配已实现，不给引擎 contract/facts/report 私加字段。结果必须绑定精确候选/base、任务和基线；绑定变化后证据失效。可信 CI 必须重新检查 merge queue 的精确合并候选。

## 当前本地接口

使用 `cargo build --locked --bin specguard` 构建，`cargo run --locked -- --help` 查看真实参数。

~~~sh
specguard doctor ROOT POLICY.json
specguard scan ROOT POLICY.json BINDING.json REQUIRED.json
specguard trace ROOT POLICY.json BINDING.json REQUIRED.json
specguard diff ROOT POLICY.json BINDING.json BASELINE.json --unverified-baseline
specguard check ROOT REQUEST.json [--cancel] [--report-dir PRIVATE_DIR]
~~~

精确约定见 [CLI 文档](docs/cli.md)与[实际进程测试](tests/cli_contract.rs)。保留 trace-check/trace-export。check 实测 0 ALLOW、2 BLOCK、新增显式版本化 baseline-review profile 的 3 REQUIRE_APPROVAL，以及 4 错误/取消；结构发现仍保持 Enforce-only。JSON 写 stdout，诊断写 stderr。`--report-dir` 通过 GE 只发布不可覆盖的 envelope receipt，完整 bundle 仍在 stdout，原始工件须独立保存并经授权解析。任意文件 `--report` 仍不支持；旧文件不代表本轮成功。此本地 CLI 不认证候选、控制器或基线。

明确版本的 Markdown 和固定 OpenSpec 来源已实现并有测试；详见[实施证据](docs/implementation-progress.md)。Spec Kit、Superpowers 及其他外部插件仍是未验证目标。原技术设计中的 flag 形式命令属于目标，不是当前语法。MCP、可信门禁使用及正式安装分发仍待完成。读取来源不会安装工具、下载依赖、执行文档指令或发放批准。

## 交付与文档

交付顺序为只读发现（S0）、规格图与身份检查（S1）、受保护基线比较（S2）、跨守卫证据（S3）、稳定 CLI/MCP/CI 与缓存兼容性（S4）。每条强制规则都需要合法、违规、工具失败、覆盖不足四类 fixture。当前已运行原生解析、快照、领域/GE 工件、运行时取消及 CLI 进程测试，详见[实施记录](docs/implementation-progress.md)。

- [架构与 ADR](docs/architecture.md)：边界、规格图、基线及运行状态、信任、集成与场景。
- [技术设计](docs/technical-design.md)：规划模块、DTO、算法、接口、诊断与可测量验收。
- [共享集成合同草案](docs/integration-contract.md)：与引擎线格式分离的编排绑定。

待完成事项包括真实审批提供方、认证候选/门禁集成、完整 CLI 决策与原子发布、进一步资源能力验证、MCP 及正式分发。本地二进制不构成托管服务或签名证明。参见 [Guard 项目仓库](https://github.com/orgs/full-stack-plugins/repositories)。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/add-specification-baseline-analysis/proposal.md)、[design](openspec/changes/add-specification-baseline-analysis/design.md)、[规范](openspec/changes/add-specification-baseline-analysis/specs/) 与 [tasks](openspec/changes/add-specification-baseline-analysis/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。任务在独立审查后逐项登记，当前检查点为19/30。新切片审查前不勾选，CLI 仍为 partial，未完整实现的能力仍标 partial。历史源码清单和验证记录继续保留为历史证据，不再作为当前能力说明。

本地 Git SDK 见[精确提交来源绑定](docs/git-source-binding.md)。当前工作区明确要求 Rust1.90，并依赖已审查的 Unix GitGuard；已实际运行 Rust1.90 库/Git 路径测试。现有 CLI 仍为本地 advisory profile。
