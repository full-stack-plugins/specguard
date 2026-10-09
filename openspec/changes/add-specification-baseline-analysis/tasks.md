# Specification Baseline Analysis Implementation Plan

> **For agentic workers:** 后续实现使用 `superpowers:subagent-driven-development` 或 `superpowers:executing-plans` 按任务推进；后续已授权实现；根协调者已验收 14/30 项（含 7deaf72 的 native parser 1.4），新切片须独立复核后登记。

**Goal:** 建立只读规格图、可信批准基线、冻结验收义务和精确候选绑定证据。

**Architecture:** SpecGuard 拥有来源解析、图、领域规则及基线语义，GuardEngine 仅执行通用 Contract/Rule/Evidence。独立 envelope 包装运行绑定，外部可信控制器认证生产者及批准；测试义务与测试结果分离。

**Tech Stack:** 拟采用 Rust 2024、Serde、Clap、Markdown AST；所有依赖/版本尚未安装或锁定。首期内存图与无持久缓存；MCP/存储/审批提供方后续经任务决策。

**Spec:** [Baseline requirements](specs/specification-baseline-analysis/spec.md)、[Evidence requirements](specs/specification-evidence-production/spec.md)、[本变更设计](design.md)、[架构](../../../docs/architecture.md)、[技术设计](../../../docs/technical-design.md)、[集成草案](../../../docs/integration-contract.md)、[跨项目路线图](../../guard-roadmap.md)。

## Global Constraints

- 基线 HEAD `0adeb0fdda9b6fb97086fe783cbf5188736c8bce` 无源码、测试或旧 OpenSpec change；下面所有实现/测试路径均为**未来拟建**，不是现有文件。
- 保持 `guard.partme.ai/v1alpha1` 严格字段和精确 `forbid_relation`；`guard.integration/v1alpha1` 是独立草案，无隐含 N/N-1 兼容。
- 默认只读、无网络、不安装或执行来源指令；批准由外部可信控制器发放/核验，不能覆盖 partial、工具错误或改写专家报告。
- 所有 enforce 规则需要合法、违规、工具故障、覆盖不足 fixture；先写失败 fixture，再实现，并记录真实结果；计划文档存在不代表完成。
- 本地发现与图可以并行引擎工作；互操作等 GE-CONTRACT/ADAPTER，可信使用等 GE-TRUST，生产独立分发等 GE-RELEASE。
- 组 1–2 产出 SG-BASELINE；依赖是阶段门，不是整仓完成屏障。组 3 的适配接口不得修改 CodeGuard 原生行为。

## Review Focus

- 非 UTF-8/未知来源版本：明确未支持，不能静默跳过；由 1.3/1.4 验证。
- 同 ID 多来源及循环映射：保留冲突并有界结束；由 1.5/1.6 验证。
- 部分解析导致的假删除：只在完整比较范围判删除；由 2.3 验证。
- 旧 queue candidate 与晚完成交错：历史可保留但不能污染当前结果；由 3.5/3.6 验证。
- 取消时旧报告文件仍存在：不能被识别为本次成功；由 3.3/4.1 验证。

## 1. Source graph foundation — SG-BASELINE first half

**Interfaces:** `discover(root, sourcePolicy) -> SourceInventory`；`freeze(inventory, candidateBinding) -> SourceSnapshot`；`parse(snapshot, adapterProfile) -> ParseResult`；`build_graph(parseResult) -> SpecificationGraph`。类型归未来 `src/model.rs`，错误/覆盖不丢失。组 1 可独立于 GE 开始。

- [x] 1.1 在 `src/model.rs`、`schemas/specguard-domain/` 定版 SourceRef/Inventory/Snapshot/ParseResult/Requirement/Acceptance/TraceEdge DTO 与显式版本字段，并建立 `Cargo.toml` 最小锁定依赖；`tests/model_contract.rs` 验证必需字段、未知版本、确定性编码。对应 “Frozen source scope and bounded parsing”“Stable requirement identity and typed trace graph”；验收为合法/缺字段/未知版本样本结果固定。
- [x] 1.2 在 `src/source.rs` 实现 discover 与权威源冲突诊断，明确只支持配置列出的根/版本；`tests/source_discovery.rs` 比较运行前后文件摘要并测试有源/无源/冲突/越界。对应 “Read-only versioned source discovery”；验收为零源文件改动且所有来源有终态。
- [x] 1.3 在 `src/source.rs` 实现 freeze、dirty 内容摘要与读取漂移检测；`tests/source_snapshot.rs` 覆盖源突变、符号链接、不同 Git 对象格式及错误编码。对应 “Frozen source scope and bounded parsing”；验收为拒绝混合快照且不假造 OID。
- [x] 1.4 在 `src/parser.rs`、`adapters/markdown/`、`adapters/openspec/` 固定一个明确格式版本并实现逐源能力/覆盖；用 `fixtures/source-versions/`、`tests/parser_coverage.rs` 测试合法、坏 YAML、未知版本、非 UTF-8、资源超限。对应 “Read-only versioned source discovery”“Frozen source scope and bounded parsing”；验收为版本矩阵仅列实测支持、预算写入配置且超限不完整。
- [ ] 1.5 在 `src/graph.rs` 实现 namespace/ID 唯一性、关系类型/方向和来源定位；`tests/graph_identity.rs` 验证重复项不会被覆盖、断链可定位、输入乱序结果一致。对应 “Stable requirement identity and typed trace graph”；验收为每个冲突源均可查。
- [x] 1.6 在 `src/graph.rs` 实现显式 ID 映射校验与有界关系索引，默认不推断重命名；`tests/graph_migration.rs` 验证双射冲突、映射环、合法移动和无映射删除/新增。对应 “Stable requirement identity and typed trace graph”；验收为所有环样本有限结束且无错误身份合并。
- [x] 1.7 在 `src/rules.rs` 实现 validate_graph 与冻结必查集合；`tests/structural_rules.rs` 为重复/引用/验收各建立合法、违规、工具故障、覆盖不足 fixture。对应 “Frozen deterministic acceptance obligations”；验收为必需 ID 一个不漏、未解析目标不伪装缺失。

## 2. Approved baseline and obligation handoff — SG-BASELINE completion

**Interfaces:** `ApprovalValidationPort` 接收不可变基线与外部引用，返回认证上下文/不可核验错误；`compare_baseline(approved, candidateGraph) -> BaselineDiff`；`impact(diff, graph) -> ImpactSet`；`export_obligations(graph, approved, scope) -> ObligationSet`。可用显式测试替身启动，生产认证依赖 GE-TRUST；native 输出不需要等待整个引擎完成。

- [x] 2.1 在 `src/baseline.rs`、`schemas/specguard-domain/baseline.json` 定义 ApprovedBaseline 与 proposed/approved/superseded/expired/revoked 状态消费，不提供批准写入；`tests/baseline_contract.rs` 检查 revision/digest/policy/scope/effective period 绑定。对应 “Authenticated immutable approved baseline”；验收为缺绑定与候选布尔批准不能构成基线。
- [x] 2.2 在 `src/integration/approval.rs` 定义只读 ApprovalValidationPort 并实现显式测试替身；`tests/approval_validation.rs` 覆盖伪造身份、范围错配、过期、撤销、服务不可用。对应 “Authenticated immutable approved baseline”；验收为无法核验和已确认未授权分开，测试批准不可进入生产 profile。
- [x] 2.3 在 `src/baseline.rs` 实现按稳定 ID 比较新增/删除/移动/引用变化；`tests/baseline_diff.rs` 覆盖完整删除与同样输入的 partial 变体。对应 “Conservative baseline diff and impact”；验收为 partial 不产生确定删除结论，原基线不改动。
- [x] 2.4 在 `src/baseline.rs` 固定首个结构化条件类型/单位比较合同；`tests/condition_comparison.rs` 验证收紧/放宽、单位不匹配、自由文本重述。对应 “Conservative baseline diff and impact”；验收为仅审核类型输出强弱，自由文本进入 review，不得依赖模型分数 enforce。
- [x] 2.5 在 `src/graph.rs` 实现 impact 的反向索引和代表路径；`tests/impact_paths.rs` 验证多路径/环/超预算与稳定排序。对应 “Conservative baseline diff and impact”；验收为每个受影响节点有可解释路径、遍历有界且不报告测试已失败。
- [x] 2.6 在 `src/obligations.rs`、`schemas/specguard-domain/obligations.json` 定版 export_obligations，输出 ID、来源/基线/候选摘要、范围和覆盖；`tests/obligation_export.rs` 验证完整导出、缺验收、partial 和摘要漂移。对应 “Versioned frozen TestObligation export”；验收为无 testPassed 虚构字段，partial 不能声明完整计划。
- [ ] 2.7 在 `fixtures/handoffs/`、`tests/baseline_handoff.rs` 固定 SG-BASELINE golden fixtures 与测试命令，记录 TestGuard/ArchGuard 消费的版本和必需字段；对应 “Versioned frozen TestObligation export”“Authenticated immutable approved baseline”；验收为消费者模拟明确标注、实际消费另有记录，稳定 ID/不可变基线/义务三项全部可验证才交付阶段门。

## 3. Candidate-bound producer — GE-CONTRACT/ADAPTER then GE-TRUST

**Interfaces:** `project(findings, protectedMapping, coverage) -> EngineInputs`；`bind(invocation, snapshot, producer, frozenScope) -> RunBinding`；`emit(binding, outcome) -> EnvelopeOrDiagnostic`。版本 schema/错误传输和 golden vectors 由 [GuardEngine change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts) 组 1–3 限定，禁止复制领域策略到引擎。

- [x] 3.1 在 `src/evidence.rs` 实现受保护的精确关系投影和映射覆盖清单；`tests/engine_projection.rs` 验证未知字段/算子拒绝、空/未映射 enforce 发现失败关闭。对应 “Protected exact-relation engine projection”；验收为每个强制发现有精确合同匹配，GE-CONTRACT/ADAPTER golden vectors 通过后才宣称互操作。
- [x] 3.2 在 `src/integration/envelope.rs` 接入独立 envelope schema、版本/能力声明与 contract/facts/report/domain 引用；`tests/envelope_contract.rs` 验证未知版本/字段、缺附件、decision/report 不同。对应 “Bound envelope and distinct failure transport”；验收为严格拒绝不匹配且不向 v1alpha1 引擎对象添加字段。
- [ ] 3.3 在 `src/integration/run.rs` 实现绑定前诊断和绑定后 completed/error/cancelled 状态；`tests/run_state.rs` 覆盖参数错误、崩溃、取消、partial、旧产物。对应 “Bound envelope and distinct failure transport”；验收为绑定前无 envelope，后置错误 decision=null，合法 partial 为 BLOCK/INDETERMINATE。
- [ ] 3.4 在 `src/integration/approval.rs`、`tests/report_immutability.rs` 对接 GE-TRUST 认证端口并验证批准引用不重写专家 decision；对应 “Bound envelope and distinct failure transport”“Authenticated freshness and audit references”；验收为 REQUIRE_APPROVAL 保持原值，FlowGuard 新报告具有自己的 scope，partial/tool error 不能被批准修复。
- [ ] 3.5 在 `src/integration/binding.rs` 实现完整 invocation/源/基线绑定和实际对象校验；`tests/queue_binding.rs` 验证 branch HEAD 与 synthetic queue candidate 差异、base/merge group 变化。对应 “Exact candidate binding and concurrent isolation”；验收为精确干净冻结候选才能作为可信门禁输入。
- [ ] 3.6 在 `src/integration/freshness.rs` 实现完整绑定幂等键、新 runId 重试和当前指针比较更新；`tests/concurrent_runs.rs` 同时运行两需求及晚返回旧候选。对应 “Exact candidate binding and concurrent isolation”；验收为无交叉满足，晚完成仅追加历史。
- [ ] 3.7 在 `src/integration/freshness.rs`、`tests/evidence_invalidation.rs` 实现并验证 candidate/base/queue、policy、analyzer/coverage、baseline、approval expiry/revocation 全部失效条件；对应 “Authenticated freshness and audit references”；验收为逐项变更均拒绝旧证据，明确 approval 无 TTL 不免除撤销/输入失效。
- [ ] 3.8 在 `src/integration/audit.rs` 实现附加式身份/绑定/摘要/因果记录及授权 evidence URI 解析；`tests/audit_integrity.rs` 检验篡改附件、伪生产者、越界 URI 和秘密脱敏。对应 “Authenticated freshness and audit references”；验收为 verify 重算不被当成信任，未选提供方前生产信任 profile 不启用。

## 4. Interfaces, rollout and joint validation — S4 / END-TO-END

**Interfaces:** `doctor/scan/trace/diff/check` 为拟议只读命令；MCP 只查询/检查；CI 消费绑定产物。组 4 生产独立分发依赖 GE-RELEASE，联合场景等待相关 SG/AG/CG/TG/GG/FG 阶段门而非互相要求整仓完工。

- [ ] 4.1 在 `src/cli.rs` 定版五个命令输入/输出、非 check 退出合同及原子报告写入；`tests/cli_contract.rs` 验证 check 0/2/3/4、取消 4、JSON stdout/诊断 stderr 分离及旧文件不算新成功。对应 “Read-only interfaces and safe execution limits”；验收为 help 与真实 flags 一致，不凭空承诺 --report。
- [ ] 4.2 在 `src/source.rs`、`src/parser.rs` 实施经过测量的路径/文件数/字节/AST/时间预算；`tests/security_limits.rs` 验证提示注入文本、绝对路径、symlink、深递归和超时。对应 “Read-only interfaces and safe execution limits”；验收为零脚本执行/越权写入且预算缺口可诊断。
- [ ] 4.3 在 `src/cache.rs` 实现可关闭的派生缓存，键包括源/解析器版本/policy/coverage/baseline；`tests/cache_parity.rs` 比较开关缓存及每项键变化。对应 “Read-only interfaces and safe execution limits”；验收为结果相同、失配重算、审批核验不被跳过。
- [ ] 4.4 在 `src/mcp.rs` 定义发现/检查/证据查询工具及认证/取消合同，选定依赖后才锁版本；`tests/mcp_readonly.rs` 验证未知工具、未授权请求、取消。对应 “Read-only interfaces and safe execution limits”；验收为无提交/批准/合并入口。
- [ ] 4.5 在 `examples/ci/`、`tests/ci_protected_policy.rs` 实现固定分析器/受保护合同/精确候选的 CI 演练，可选 SARIF 仅定位；对应 “Protected exact-relation engine projection”“Exact candidate binding and concurrent isolation”；验收为候选修改规则无效、缺可信绑定/覆盖的报告不被消费。
- [ ] 4.6 在 `docs/compatibility.md`、`fixtures/compatibility/`、`tests/compatibility_matrix.rs` 冻结 source/SDK/envelope/profile 实测矩阵及独立分发策略；对应 “Phased compatibility rollout and reversible rollback”；验收为未知能力失败关闭，无隐含 N/N-1，固定源码可开发而生产包等待 GE-RELEASE。
- [ ] 4.7 在 `examples/rollout/`、`tests/rollout_rollback.rs` 实现 advisory→shadow→opt-in→enforce 的显式配置和适配器独立回退；对应 “Phased compatibility rollout and reversible rollback”；验收为历史附件/源文档保留，必需能力回退后不假放行，CodeGuard 原生命令和退出码不变。
- [ ] 4.8 在 `fixtures/end-to-end/`、`tests/joint_gate.rs` 与相关守卫执行两个并行需求、精确 synthetic queue candidate、审批过期/撤销、基线漂移、晚完成和回退联合场景；对应 “Phased compatibility rollout and reversible rollback”“Exact candidate binding and concurrent isolation”；验收为保存真实对端版本/命令/结果、引用 CodeGuard native parity 证据，不用模拟结果或 OpenSpec 文档验证勾选完成。

## Local reviewed acceptance checkpoint

Independently reviewed local implementation accepts tasks 1.1, 1.2, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6. See docs/implementation-progress.md for evidence. These checkmarks cover the explicitly supported local profiles, not production authentication, hosted enforcement or release. Other tasks remain pending.
