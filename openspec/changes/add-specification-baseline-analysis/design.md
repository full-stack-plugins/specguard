## Context

检查基线为 `0adeb0fdda9b6fb97086fe783cbf5188736c8bce`；该提交仅有 README 与 docs，无产品源码、测试、依赖清单或旧 OpenSpec change。历史 main 证据见 [架构](../../../docs/architecture.md)。本变更把[技术设计](../../../docs/technical-design.md)的 S0–S4 转为新增要求，不宣称已实现。当前引擎协议行为来自共享基线，未在 SpecGuard 本地验证 SDK。

## Goals / Non-Goals

目标：建立只读、完整性可解释的规格图，认证不可变批准基线，导出冻结验收义务，产生绑定精确候选的可复算证据。目标消费者为开发者、TestGuard、ArchGuard、GitGuard 和 FlowGuard。

非目标：自动理解或批准业务语义、执行行为测试、修改源文档、安装外部插件、合并/发布。OpenSpec、Spec Kit、Superpowers 和历史工作流插件仍是未验证兼容目标。引擎不接收领域解析器、审批提供方或流程阶段。

## Decisions

### 1. 模块、来源合同与范围

未来 `src/source.rs` 定义 `discover(root, sourcePolicy) -> SourceInventory` 和 `freeze(inventory, candidateBinding) -> SourceSnapshot`；`src/parser.rs` 定义 `parse(snapshot, adapterProfile) -> ParseResult`。`src/model.rs` 定义这些内部 DTO；格式与字段通过本变更任务审核后定版，不直接变成引擎字段。

`SourceInventory` 包含显式根、来源格式/版本、必查清单与冲突；`SourceSnapshot` 绑定实际字节摘要和不可变候选/base，dirty 本地输入另外绑定源摘要。`ParseResult` 包含节点、关系、SourceRef 及逐源完成/未解析原因。每个必查来源必须有终态；双权威源、未知版本、读取中漂移不可静默成功。

先建立严格显式 ID Markdown fixture，再选择一个固定 OpenSpec 格式版本。版本支持矩阵用真实样本验证，不以读取通用 Markdown 宣称支持所有工具。文件数/字节/AST 深度/遍历/时间预算在组 1 决定且 fixture 测量；超限为 partial 或 error。

### 2. 身份、图与结构化规则

`src/graph.rs` 的 `build_graph(parseResult) -> SpecificationGraph` 保留 namespace/稳定 ID、原始位置与重复项；不能用标题/行号生成永久身份，也不能“最后一个覆盖”。关系具有类型、方向和循环策略，索引和 visited 集合使遍历有界。

`src/rules.rs` 的 `validate_graph(graph, frozenObligations) -> DomainFindings` 确定性检查重复、断链、必需验收；输入集合在运行前冻结。未知覆盖不能等价于“缺失引用/删除”。`src/baseline.rs` 的 ID 映射默认关闭；经审查的双射无环映射才允许重命名，否则删除/新增。

### 3. 认证基线、差异及义务导出

内部 `ApprovedBaseline` 绑定仓库/范围、不可变 source revision/content digest、policy digest、approvalRef 与有效期；`ApprovalValidationPort` 由外部可信控制器核验身份、范围、新鲜度和撤销，领域代码不持有签名或批准凭据。测试替身不是生产认证。

`compare_baseline(approved, candidateGraph) -> BaselineDiff` 对完整范围确认新增/删除/移动/引用变化；仅已审核结构化条件可比较收紧/放宽，自由文本摘要变化进入 review。无法认证审批是无法完成必要分析；已完整确认未授权删除才是 enforce 违例。`impact(diff, graph) -> ImpactSet` 输出受影响节点及确定性代表传播路径，不冒充测试失败。

`export_obligations(graph, approved, scope) -> ObligationSet` 位于未来 `src/obligations.rs`，输出稳定 obligation/requirement/acceptance IDs、来源/基线/候选摘要、必查范围、覆盖和关联 ADR/任务。它是独立版本化领域附件，没有 testPassed；部分范围不得作为冻结的完整验收计划。组 1–2 的本地 fixture 和输出合同构成 SG-BASELINE；生产认证仍等 GE-TRUST。

### 4. 协议、映射和状态

当前严格 `guard.partme.ai/v1alpha1` 仅有 GuardContract YAML、GuardFacts JSON、GuardReport JSON 及精确 `forbid_relation`；不接受任意域字段。`src/evidence.rs` 将完整领域发现投影为受保护规则覆盖的精确关系，逐条证明无未映射强制发现；集合量化留在 SpecGuard。GE-CONTRACT/GE-ADAPTER 完成前只使用标明模拟的 fixture。

独立 [GuardRunEnvelope 草案](../../../docs/integration-contract.md)版本 `guard.integration/v1alpha1`，与 crate semver、policy revision、analyzer version 分开。严格读者拒绝未知字段/版本，兼容矩阵只列实际验证组合，不暗示 N/N-1。

运行 `discovered → frozen → parsed → normalized → compared → evaluated → emitted`。在 binding、producer、必查覆盖冻结前失败，只输出独立诊断，不造空 OID/envelope。冻结后错误/取消可输出 `runStatus=error/cancelled`、`decision=null`；合法 partial facts 由引擎评估为 completed/BLOCK/INDETERMINATE。引擎完成的 envelope 必须引用 contract/facts/report，decision 等于 report；外部批准不能改写 REQUIRE_APPROVAL，也不能修复 partial/tool error。FlowGuard 只能在自己的新范围生成自己的门禁报告。

目标 `check` 退出 0/2/3/4；取消适配为 4，信号可能无法输出。stdout 为完整 JSON，stderr 为诊断。`doctor/scan/trace/diff` 的退出合同和最终 flags 在组 4 冻结，不把此设计当作存在的 CLI；没有确定的 `--report` 行为。

### 5. 精确绑定、信任与审计

未来 `src/integration/binding.rs` 验证 repo/task/worktree/requirementIds/candidateOid/baseOid/mergeGroupId/sourceSnapshotDigest/baselineDigest；OID 按仓库对象格式验证，不硬编码长度。可信门禁读取干净冻结的实际 queue candidate，分支 HEAD 证据不够。

完整绑定和输入摘要作幂等键，重试使用新 runId；未来 `src/integration/freshness.rs` 在 candidate/base/queue、policy、analyzer/coverage、baseline、approval expiry/revocation 变化时使结果失效。不同需求任务隔离；晚完成只能追加历史，以绑定比较更新当前指针。未来 `src/integration/audit.rs` 追加已认证 actor、binding、摘要、policy/approval refs、结果、因果前驱和可信时间；默认不保留全文/秘密。

GE-TRUST 提供通用引用核验端口，外部控制器发放身份与批准；SpecGuard定义领域基线含义。报告无签名，verify 只重算，不证明可信来源或合并权。

### 6. 安全、缓存与外部入口

默认只读、无网络、不执行来源中的命令/策略；限制符号链接/路径穿越/输出路径与资源量。审批适配器仅在显式启用时使用只读认证能力。文件输出原子提交，不留下可误读为成功的旧报告。首期无持久缓存；后续 `src/cache.rs` 的 key 包含源、分析器/版本、policy、coverage、baseline，不缓存审批有效性来跳过实时核验。

MCP 仅发现/检查/证据查询；CI 使用受保护规则和固定分析器；SARIF 只是后续定位投影，不能替代覆盖证明。存储、rmcp、CI 平台和审批提供方均需单独决定，不能默认已接入。向 ArchGuard 只输出需求/ADR 引用，向 TestGuard 输出义务；跨守卫的实现不放在本仓库。

## Dependencies

依照[跨项目路线图](../../guard-roadmap.md)，组 1–2 的 SG-BASELINE 可独立启动。组 3 可互操作投影等待 [GuardEngine GE-CONTRACT/GE-ADAPTER](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)，可信审批/证据等待 GE-TRUST；独立生产分发等待 GE-RELEASE，开发可固定源码版本。

[TestGuard](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline)的需求计划和 [ArchGuard](https://github.com/full-stack-plugins/archguard/tree/docs/guard-design-20261009/openspec/changes/extend-architecture-analysis-and-evidence)的规格/ADR 关联消费 SG-BASELINE；基础解析/扫描可并行。GitGuard 只读候选绑定先于 FlowGuard 组合门禁，SpecGuard 不依赖其特权执行器，避免循环依赖。

## Migration and rollback

S0/S1 完成本地发现/图；S2 完成批准基线和义务；S3 完成受版本约束的证据；S4 稳定接口、缓存、CI/MCP。部署按 advisory capture → shadow comparison → opt-in protected checks → 显式 enforce 推进，每阶段保留实际输出、固定版本和负向 fixture。未完成验证时不得声称门禁有效。

回退仅关闭/回退 SpecGuard 适配器或控制器配置，保留不可变历史附件和原始规格；不能删除策略要求的义务让旧结果变绿。降级成不支持能力时，必需门禁失败关闭。不得改动 CodeGuard 原生命令/退出码，也不要求整个旧 CodeGuard change 完成才能对已证明范围协作。

## Risks / Open Questions

- 来源版本与配置语法：组 1 以一个显式版本作为可逆默认；其他工具保持未支持。
- ID 迁移与结构化条件类型：默认不猜身份/语义；开启类型前必须有正反例和审核。
- 摘要规范化/SDK 版本：由 GE-CONTRACT/ADAPTER golden vectors 约束；未冻结前禁用互操作声称。
- 审批提供方、存储、保留期限：组 3 先定义端口和测试，生产提供方必须经认证审查，不自签授权。
- 资源预算/吞吐：组 1 测量后固定上限；无性能 SLA 声称。

## Validation strategy

每个 enforce 有合法、违规、工具故障、覆盖不足四类 fixture；另测编码/版本、循环、身份迁移、文本 review、审批伪造/过期/撤销、摘要篡改、队列候选变化和晚完成。联合 END-TO-END 在相关门完成后使用两个并行需求、精确 synthetic queue candidate、漂移和回退；CodeGuard native parity 使用对端结果而不复制其原生实现。文档语法验证不算运行时验收，也不会勾选任务。
