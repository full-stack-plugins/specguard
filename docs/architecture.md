# SpecGuard — 规格守卫架构设计

> Partme Guard | 版本 V0.1 | 状态：目标架构，尚未实施 | 2026-10-09

## 1. 目标、定位与边界

SpecGuard 治理“原始诉求 → 产品需求 → 功能规格 → 验收条件 → 设计/任务 → 测试义务”的可追踪性、完整性和演进。它的作用不是让 AI 自动批准需求，而是把已批准的规格基线、结构要求、引用和影响关系变成**可执行的确定性约束**。

职责包括：规格来源发现、格式化事实提取、稳定身份/引用检查、缺失的验收义务、基线差异、变更影响和规格审查证据。它不拥有：需求商业取舍、审批权限（FlowGuard）、架构职责归属（ArchGuard）、测试执行（TestGuard）、Git 合入（GitGuard）。

**判断等级**：重复 ID、悬空引用、未经批准删除验收条件可以 ENFORCE；业务语义歧义、需求是否真正理解正确必须 REVIEW；改善措辞的意见属于 ADVISE。不能把另一个 LLM 的分数伪装成确定性证明。

## 2. 组件边界与数据流

~~~text
OpenSpec / Spec Kit / Superpowers / explicit Markdown
            │
            ▼
Source Adapter (read-only, source + digest + coverage)
            │
            ▼
Specification Graph
 OriginalRequest → Requirement → AcceptanceCriterion
                         │
                   Design / Task / TestObligation
            │
    ┌───────┴────────┐
    ▼                ▼
Structural Rules   Baseline / Diff / Impact
    └───────┬────────┘
            ▼
GuardFacts + Findings + Provenance + Coverage
            │
         GuardEngine
  Contract / Rule / Evidence
            │
          Result
            ▼
    CLI / MCP / CI / FlowGuard
~~~

### 2.1 Source Adapter

必须显式报告来源格式、版本、文件、行范围、内容摘要、已解析节点和未解析范围；未知格式或双权威来源不自动选“最新”，返回 INDETERMINATE 并请求确定唯一基线。只读检查不隐式初始化 OpenSpec、下载工具或修改原始 Markdown。

### 2.2 Specification Graph

节点：OriginalRequest、Requirement、AcceptanceCriterion、ArchitectureDecision、Task、TestObligation、SourceRevision。关系：refines、satisfies、verified_by、depends_on、supersedes、approved_from。ID 基于项目命名空间稳定化，支持重命名映射；图谱索引是事实投影，不是第二套需求主数据。

### 2.3 Baseline and Change Impact

批准规格来自受信版本/审批引用；候选文档内写入 `approved: true` 不构成授权。比较新增、删除、重述、收紧、放宽、移动、引用重定向；将可能影响的架构规则、任务与测试义务关联下游证据失效。

## 3. GuardEngine 协议

沿用 [Guard Protocol v1alpha1](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md) 的 GuardContract、GuardFacts、GuardReport 和 enforce/review/advise。关系事实示例：`REQ-017 has_acceptance AC-017-1`、`TASK-024 implements REQ-017`。**当前引擎只实现精确 forbid_relation 算子**；诸如“每个 Requirement 至少有 N 个 Acceptance”必须由 SpecGuard 领域验证器确定性计算后再输出结构化事实，或通过正式协议升级支持，不能假定 YAML 字段自动生效。

执行状态、规则结论、覆盖范围分离。分析器不能解析所有目标文件时报告 partial，必需检查 INDETERMINATE、最终不能 ALLOW。局部检查通过≠工程整体交付允许。

## 4. 信任边界与协作

候选文档、编码 Agent、工作区扫描结果均不可信；可信 CI 需要用受保护契约基线、当前 Git Tree、真实解析器版本和任务批准范围重新分析，关联候选 SHA。签名与合入授权不由 SpecGuard 发放。

SpecGuard 对接 spec-workflow-plugin 的候选规格；ArchGuard 消费批准需求/ADR 引用；TestGuard 消费冻结验收义务；FlowGuard 核验真实人工批准；GitGuard 核验规格变更是否越权并检查目标分支版本。

## 5. 架构决策（ADR）

- SG-ADR-001：读取已有项目规格源，不新增第二份权威需求正文。
- SG-ADR-002：只有机器可证明的结构/版本违规 ENFORCE，业务理解差异交由 REVIEW。
- SG-ADR-003：验证义务在检查前冻结，不能从通过的子集反推全部要求。
- SG-ADR-004：默认只读，不自动 fetch/install/init。
- SG-ADR-005：六 Guard 独立运行，组合门禁归 FlowGuard，通用引擎归 GuardEngine。

## 6. 最小验收矩阵

| 情形 | 期望 |
|---|---|
| 有效规格、完整引用、批准基线未变化 | 指定范围 PASS |
| 重复 ID、断链、缺少强制验收 | 精确定位并 FAIL |
| 无授权删除/放宽已批准验收 | 保留基线，BLOCK |
| 多个互斥规格权威来源 | 明确冲突，不自行选择 |
| 部分源未解析、解析器失败或事实过期 | partial / INDETERMINATE / BLOCK |
| AI 在候选规格中伪造批准标识 | 不授予审批权限 |
| 修改规格基线后重用旧测试证据 | 证据失效，重新验证 |

本文件描述目标架构，详见 [技术方案](technical-design.md)，不宣称已实现 CLI/MCP。
