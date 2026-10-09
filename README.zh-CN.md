# SpecGuard — 规格守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**面向 AI 原生研发的确定性需求、规格与验收治理。**

> **当前仅有设计文档。** 本次检查基于 `main` 提交 `01137804aa465c1b931c73c9d211ff0d29b46c7a`（检查日期 2026-10-09）。已跟踪文件只有两份 README 与两份设计文档，没有源码、包清单、测试、schema、CI 配置或 OpenSpec 工作区。下述 SpecGuard 能力与命令全部是目标方案，不代表已经实现。文档描述方向，不代表实施或验证已完成。

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

输入为显式选择的规格源、候选快照、受保护策略，以及需要时的基线和经外部身份系统核验的审批记录。规划输出包括规格图、精确定位的发现、基线差异、覆盖清单以及兼容引擎的事实/报告引用。测试义务表示需要获得什么证据，不代表行为测试已经通过。

SpecGuard 负责规格解析和领域检查；ArchGuard 负责架构检查；CodeGuard 负责代码检查；TestGuard 负责测试证据；GitGuard 负责 Git 检查；FlowGuard 编排门禁及可信审批核验。六个守卫独立使用 [GuardEngine](https://github.com/full-stack-plugins/guardengine)，由引擎负责通用合同校验、中立规则评估和确定性证据计算。SpecGuard 和引擎都不授予业务批准、合并或发布权限。

## 决策、覆盖与兼容

确定性违例可以映射为 `enforce`；语义不确定性采用 `review`；建议采用 `advise`。引擎决策为 `ALLOW`、`BLOCK`、`REQUIRE_APPROVAL`。partial 事实意味着 `BLOCK` 和 `INDETERMINATE` 评估，不能表示完整检查通过。“完整”仅指声明的分析器范围，不代表理解了全部业务。人工批准不能覆盖分析失败或覆盖不足。

共享的当前 `guard.partme.ai/v1alpha1` 协议仅支持 GuardContract YAML、GuardFacts JSON、GuardReport JSON 和精确 `forbid_relation` 断言，拒绝未知字段。图集合量化检查应放在规划的 SpecGuard 领域验证器中；来源位置、审批和编排信息不能私自添加为引擎字段。报告无签名；verify 只重算结果，不证明信任或授权。以上是共享引擎兼容基线，并非本仓库已验证的 SpecGuard 集成。

规划的[集成合同](docs/integration-contract.md)使用独立的 `GuardRunEnvelope`（`guard.integration/v1alpha1`，**草案**），不是当前引擎接受的扩展。结果必须绑定精确候选/base、任务和基线；绑定变化后证据失效。可信 CI 必须重新检查 merge queue 的精确合并候选。

## 规划接口——尚不可执行

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017 --format json
specguard diff --base <approved-ref> --head HEAD
specguard check --project . --format json
~~~

目前没有实现这些命令的二进制或安装器。`check` 目标退出码为：`0` ALLOW、`2` BLOCK、`3` REQUIRE_APPROVAL、`4` 输入/运行/验证错误；其他命令的精确退出合同仍需定义。规划机器输出写入 stdout，诊断写入 stderr。没有已实现的 `--report` 参数。MCP 与 CI 属于后续目标，也尚未实现。

OpenSpec、Spec Kit、Superpowers 及历史规格工作流插件适配均为**未经验证的兼容目标**，不意味着已经安装或成功测试外部插件。读取项目不能隐式初始化工具、下载依赖、发放批准或修改规格。

## 交付与文档

交付顺序为只读发现（S0）、规格图与身份检查（S1）、受保护基线比较（S2）、跨守卫证据（S3）、稳定 CLI/MCP/CI 与缓存兼容性（S4）。每条强制规则都需要合法、违规、工具失败、覆盖不足四类 fixture。本次没有运行运行时测试或 OpenSpec 验证，因为检查的目录中两者均不存在。

- [架构与 ADR](docs/architecture.md)：边界、规格图、基线及运行状态、信任、集成与场景。
- [技术设计](docs/technical-design.md)：规划模块、DTO、算法、接口、诊断与可测量验收。
- [共享集成合同草案](docs/integration-contract.md)：与引擎线格式分离的编排绑定。

待定事项包括支持的源格式版本、稳定 ID 迁移策略、审批提供方、摘要/schema 实现及实测资源预算。目前不宣称存在二进制、托管服务或签名证明。参见 [Guard 项目仓库](https://github.com/orgs/full-stack-plugins/repositories)。
