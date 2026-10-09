# SpecGuard — 技术实施方案

> 状态：待实现的技术基线（V0.1），不代表已有 CLI、MCP 或 CI 门禁。

## 1. 技术栈

Rust 2024、Serde、JSON Schema、按需 Tokio/Clap；Markdown AST + OpenSpec/Spec Kit/Superpowers 只读适配；首期内存图，后续按需 SQLite 索引缓存；GuardEngine SDK 负责通用契约/规则/证据机制；可选 rmcp 和 GitHub Actions。**不得将规范解析器、审批服务或图数据库硬塞进 GuardEngine。**

## 2. 模块设计

~~~text
specguard/
├── src/{model,source,parser,graph,baseline,rules,evidence,cli}.rs
├── adapters/{openspec,speckit,superpowers,markdown}/
├── schemas/v1alpha1/
├── fixtures/{valid,duplicate,broken,baseline-change,partial}/
├── docs/
└── openspec/
~~~

初期单 crate 即可，内部模块独立测试。主要 DTO：SourceRef(root,path,line,revision,digest)、Requirement(id,origin,state,textDigest)、AcceptanceCriterion(id,requirementId,observable,expected)、TraceEdge(from,relation,to)、BaselineRef(approvedRevision,approvalRef,contractDigest)、Coverage(requiredSources,observedSources,unparsedSources)。源路径标准化，hash 包含解析器及来源版本，不允许无版本缓存在修改后继续使用。

## 3. 规则 DSL 示例

~~~yaml
apiVersion: guard.partme.ai/v1alpha1
kind: GuardContract
metadata:
  id: sample-acceptance-baseline
  revision: "1"
spec:
  rules:
    - id: SPEC-001
      enforcement: enforce
      assertion:
        type: forbid_relation
        subject: AC-017
        predicate: removed_without_approval
        object: approved-baseline
~~~

SpecGuard 应把真实 baseline diff 转为对应关系事实；不能直接接受 Agent 填写的“没有违规”。集合量化/完整性由专业验证器实现，待 Guard Protocol 增加扩展点后逐步迁移，不破坏 v1alpha1 的字段约束。

## 4. 处理流水线与错误类型

1. discover：列出支持的规格来源、解析器版本和缺少的能力，不产生副作用。
2. snapshot：读取受信基线引用与候选文件清单，冻结必查 ID/来源摘要。
3. parse：读取限定范围，保存原始位置，乱码/坏 YAML/未知格式不得忽略。
4. normalize：构建 ID 唯一的引用图和精确来源映射。
5. diff：确定新增、删除、弱化、影响下游测试与架构义务。
6. validate：执行结构规则、引用规则、基线规则；语义风险单独 REVIEW。
7. emit：输出 GuardFacts、Finding、Coverage、diagnostics 供 GuardEngine 评估。
8. trusted recheck：CI 在确切合入候选使用批准契约独立重跑；不信任 PR 修改的规则。

标准化失败种类：InvalidSource、DuplicateIdentity、MissingReference、AmbiguousAuthority、BaselineDrift、ApprovalUnverified、AnalyzerPartial、ContractUnsupported。不能把工具故障误写成通过或业务失败。

## 5. 入口（规划中，尚不可运行）

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017 --format json
specguard diff --base <approved-ref> --head HEAD
~~~

MCP 初期只支持发现、检查、证据查询，不支持直接提交/批准。GitHub CI 使用受保护规则集，输出 JSON，后续可输出 SARIF。

## 6. 安全、性能和可靠性

禁止文档内提示注入触发命令；限制 symlink 逃逸、文件数、字节数、递归深度、单次解析时间；默认不将客户需求全文发到远端；缓存必须以内容摘要、解析器版本和合同摘要共同键控。多任务并行隔离 task/tenant/candidate。验证中断或缺失必须产生可恢复的 partial 证据，不向合入端冒充成功。

## 7. 实施 Wave 与验收

| Wave | 内容 | 可验收行为 |
|---|---|---|
| S0 | source doctor、SourceRef、能力声明 | 完全只读、源缺失可诊断 |
| S1 | ID/追踪图/结构与引用规则 | valid/duplicate/broken/partial |
| S2 | 可信批准基线 diff/影响图 | 删除验收、伪造批准被拒绝 |
| S3 | TestGuard/ArchGuard/FlowGuard 跨仓证据 | 变更之后旧证据失效 |
| S4 | CLI/MCP/CI、增量缓存与兼容性 | N/N-1、真实仓库与绕过验收 |

完成标准：每条硬约束必须存在合法、违规、工具故障和覆盖不足四类 fixture；只有实际执行结果和可信门禁对接可称为实施完成。
