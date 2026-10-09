## Why

SpecGuard 当前仍为文档仓库：历史 main `01137804aa465c1b931c73c9d211ff0d29b46c7a` 只有两份 README 和两份设计文档；本次规划检查的分支 HEAD 为 `0adeb0fdda9b6fb97086fe783cbf5188736c8bce`，增加了共享集成设计，但仍无源码、Cargo 清单、测试或既有 OpenSpec change。需要把设计转化为可逐项执行、可验证的增量任务，而不是将文档完成误报为产品能力。

## What Changes

- 新增只读来源发现、版本化适配器、冻结快照、稳定需求身份及类型化追踪图的目标要求。
- 新增外部认证的不可变批准基线、差异分类、影响传播及冻结 TestObligation 导出。
- 新增确定性领域发现到现有引擎精确关系的受保护投影；规格解析与政策保留在 SpecGuard。
- 新增独立 envelope、覆盖/错误状态、候选与任务隔离、审批新鲜度、审计和受限接口的目标要求。
- 规划 S0–S4 的测试、渐进启用与回退。所有实现任务保持未完成；本变更本身不创建产品源码、不安装依赖、不执行实现。

## Capabilities

### New Capabilities

- `specification-baseline-analysis`: 只读来源图、稳定 ID、审批基线、差异、影响与 TestObligation 导出。
- `specification-evidence-production`: 严格引擎投影、候选绑定、错误/覆盖合同、信任审计、CLI/MCP/CI 和渐进交付。

### Modified Capabilities

无。仓库此前没有 OpenSpec 规格；本变更不替换其他守卫的能力或既有设计文档。

## Impact

未来实现表面为 `src/{source,parser,model,graph,baseline,rules,evidence,cli}.rs`、`src/integration/`、`adapters/`、`schemas/`、`tests/` 与 `fixtures/`，目前均未创建。Rust 2024/Serde/Clap/Markdown AST 是设计候选，不是已安装依赖；审批提供方和持久化实现尚未选择。

当前 `guard.partme.ai/v1alpha1` 的 GuardContract/GuardFacts/GuardReport 严格字段、`forbid_relation` 和退出语义保持不变。`guard.integration/v1alpha1` 仍是独立草案；本计划不宣称引擎已解析该 envelope 或支持隐含 N/N-1 兼容。无 GuardCore，无批准发放、合并或发布权限。

## Dependencies and phased gates

本变更组 1–2 形成 **SG-BASELINE**：稳定 ID/图、认证不可变基线及 TestObligation 导出。只读发现与领域 fixture 可以和 GuardEngine 并行；可互操作生产者须等待 **GE-CONTRACT/GE-ADAPTER** 冻结和验证映射，可信使用须等待 **GE-TRUST**，生产独立分发须等待 **GE-RELEASE**。先使用固定 source revision 不必等待整个引擎仓库完成。

- [GuardEngine 分阶段合同与适配](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)：GE-CONTRACT、GE-ADAPTER、GE-TRUST、GE-RELEASE。
- [TestGuard 义务证据管线](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline)：消费 SG-BASELINE；导出义务不代表测试通过。
- [ArchGuard 证据演进](https://github.com/full-stack-plugins/archguard/tree/docs/guard-design-20261009/openspec/changes/extend-architecture-analysis-and-evidence)：仅需求/ADR 追踪依赖 SG-BASELINE，基础扫描不依赖它。
- [GitGuard 候选绑定](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance)及 [FlowGuard 门禁](https://github.com/full-stack-plugins/flowguard/tree/docs/guard-design-20261009/openspec/changes/add-evidence-bound-workflow-gates)：后续消费限定范围证据；不反向阻塞本地发现。

END-TO-END 需精确合并队列候选、两个并行需求、审批过期/撤销、基线漂移、晚完成隔离和回退；CodeGuard 原生行为一致性由其适配变更拥有，本仓库仅参与联合验收，不复制实现。

## References

- [架构](../../../docs/architecture.md)
- [技术设计](../../../docs/technical-design.md)
- [共享集成草案](../../../docs/integration-contract.md)
- [跨项目路线图](../../guard-roadmap.md)
- [本变更设计](design.md)与[未完成任务](tasks.md)
