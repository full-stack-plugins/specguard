## ADDED Requirements

### Requirement: Protected exact-relation engine projection
SpecGuard SHALL 将确定性领域发现投影到受保护合同覆盖的精确关系，保持当前 guard.partme.ai/v1alpha1 严格字段与 forbid_relation，MUST 不遗漏未映射强制发现或向引擎注入领域扩展字段。

#### Scenario: Unmapped enforce finding
- **WHEN** 领域校验产生无法由选定映射/合同表示的强制发现
- **THEN** 报告不支持/错误，不通过丢弃发现返回 ALLOW。

#### Scenario: Strict engine compatibility
- **WHEN** 发送未知字段/算子或合法 partial facts
- **THEN** 未知输入被拒绝；合法 partial facts 形成 BLOCK/INDETERMINATE，而不是 ALLOW。

### Requirement: Bound envelope and distinct failure transport
SpecGuard SHALL 仅在 binding、producer 和必查覆盖冻结后输出独立 guard.integration/v1alpha1 envelope；引擎完成结果的 decision MUST 等于引用报告，错误/取消 decision MUST 为 null。

#### Scenario: Failure before binding
- **WHEN** 参数、仓库或候选/base 歧义导致绑定无法建立
- **THEN** 只输出独立失败诊断，不生成 envelope、不伪造 OID 或空必需字段。

#### Scenario: Failure after binding
- **WHEN** 完整绑定冻结后工具失败或取消
- **THEN** 输出 error/cancelled 且 decision=null；合法引擎 partial 评估仍可为 completed/BLOCK。

#### Scenario: Approval cannot rewrite a report
- **WHEN** 外部批准到达一个 REQUIRE_APPROVAL 专家报告
- **THEN** 专家原报告及 envelope 决策不变；任何 FlowGuard 门禁结论属于新的自身范围报告。

### Requirement: Exact candidate binding and concurrent isolation
SpecGuard SHALL 将证据绑定 repo/task/worktree/requirement 集合、candidate/base/merge group 和源/基线摘要，MUST 在可信门禁分析实际干净冻结的队列候选，并防止并行任务交叉满足或晚结果覆盖新候选。

#### Scenario: Queue candidate changes
- **WHEN** 队列重算生成新 candidate 或 base/merge group
- **THEN** 旧分支或旧队列证据不可复用，需对精确新候选重跑。

#### Scenario: Two requirements and late result
- **WHEN** 两个需求任务并发且旧尝试晚返回
- **THEN** 结果按完整不可变绑定隔离；旧结果只追加历史，不覆盖当前指针。

### Requirement: Authenticated freshness and audit references
SpecGuard SHALL 经外部可信端口核验生产者/审批引用，检查规则、分析器/覆盖、基线及审批新鲜度并追加可追踪审计，MUST 不将 unsigned 报告或 verify 重算视为认证或合并权限。

#### Scenario: Freshness invalidation
- **WHEN** policy、analyzer/coverage、baseline 或 approval 有效性改变
- **THEN** 受影响证据失效，审计记录绑定、摘要、原因及可信身份/时间。

#### Scenario: Tampered or unauthenticated artifact
- **WHEN** 附件摘要不符或生产者不可认证
- **THEN** 必需门禁拒绝消费，不能仅凭自报身份或重算一致授予信任。

### Requirement: Read-only interfaces and safe execution limits
SpecGuard SHALL 提供分离发现/扫描/追踪/差异/检查职责的只读入口，check 使用 0/2/3/4 技术决策/错误合同并分离 JSON stdout 与诊断 stderr，MUST 限制资源与路径且不开放批准/合并接口。

#### Scenario: CLI check outcomes
- **WHEN** check 分别获得 ALLOW、BLOCK、REQUIRE_APPROVAL 或输入/运行/验证错误
- **THEN** 分别退出 0、2、3、4；取消适配为 4，JSON 不与日志混流。

#### Scenario: Hostile input and optional cache
- **WHEN** 文档含命令/路径逃逸或缓存来自不同版本/范围
- **THEN** 不执行命令或越权读取；缓存不匹配时重算，审批新鲜度不得仅依赖缓存。

### Requirement: Phased compatibility rollout and reversible rollback
SpecGuard SHALL 用真实版本/能力矩阵和每条硬约束四类 fixture 验证，按 advisory、shadow、opt-in、enforce 分阶段启用，MUST 在回退时保留不可变证据和原生外部接口，不隐藏缺失能力。

#### Scenario: Release and independent rollback
- **WHEN** 适配层升级或回退
- **THEN** 仅使用已验证兼容组合，保留原始规格/历史证据，必需能力不支持则失败关闭，不改 CodeGuard 原生接口。

#### Scenario: Joint gate acceptance
- **WHEN** 相关阶段门具备且运行联合演练
- **THEN** 使用精确 synthetic queue candidate、两个需求、过期/撤销、漂移、晚完成及回退证据；文档验证不计为实现完成。
