# SpecGuard — 规格守卫架构设计

> 目标架构，尚未实施；2026-10-09 文档审查。检查的 `main`：`01137804aa465c1b931c73c9d211ff0d29b46c7a`。以下组件、状态与接口均为设计，不是现有运行行为。

## 1. 仓库证据与目标

`git ls-tree -r --name-only HEAD` 在上述提交仅列出 `README.md`、`README.zh-CN.md`、`docs/architecture.md`、`docs/technical-design.md`。无 `src/`、Cargo 清单、测试、配置、schema、CI 或 `openspec/`，因此无可核对的解析器、CLI 帮助或 OpenSpec 实施任务。原有设计是本次保留和细化的依据，不能作为实现证据。GuardEngine 当前协议行为来自共享兼容基线；本仓库没有完成 SDK 接入或协议测试。

SpecGuard 治理“原始诉求 → 需求 → 验收 → 设计/任务 → 测试义务”的可追踪性、完整性和演进。目标是将批准规格中的结构、身份和版本要求转为确定性约束，而不是让 AI 批准需求。

主要使用者为编写需求的产品人员、实现任务的 Agent、评审者和可信 CI 控制器。开发者获得断链定位；评审者获得基线差异与待判定语义；CI 获得限定范围且可重算的证据。

| 拥有 | 不拥有 |
|---|---|
| 来源适配、稳定身份、引用和必查范围 | 自动推断真实业务意图、业务批准 |
| 验收义务、规格基线差异、影响传播 | 真实测试执行或测试通过证明 |
| 领域结构规则、覆盖与来源证据 | 通用引擎协议的任意扩展 |
| 只读检查和审查解释 | 自动安装、修改规格、合并、发布 |

确定性重复 ID、悬空引用、未经有效授权删除结构化义务可映射 `enforce`；业务歧义、自由文本含义变化使用 `review`；非必需改进使用 `advise`。人类审查不能把无法读取的文件变成已分析。

## 2. 组件与数据流

~~~text
声明的来源 + 精确候选树 + 受保护合同 + 已认证的基线引用
                             │
                 Source discovery / snapshot
                             │
                Read-only source adapters
                  │ 来源映射 + 覆盖清单
                             ▼
              Specification Graph（事实投影）
                  │                    │
          Structural validator   Baseline / diff / impact
                  └─────────┬──────────┘
                    Domain findings
                             │
              Validated projection → GuardFacts
                             │
                GuardEngine Contract / Rule / Evidence
                             │
          GuardReport + 独立的领域附件与集成 envelope
                             │
                  CLI / MCP / CI / FlowGuard
~~~

- **Source discovery** 仅枚举允许的根路径、明确支持的格式与版本。多个权威来源冲突时不按修改时间选胜者。
- **Snapshot** 冻结候选文件清单、内容摘要、必查 ID 与基线。工作区在运行中变化时不得混合读取不同版本。
- **Adapters** 提取原始位置、节点、关系与未解析范围；不执行文档中的命令或插件。
- **Graph** 统一引用与身份；正文仍归原始规格源所有，图不是第二套主数据。
- **Domain validators** 计算 ID 唯一性、引用存在性、最小验收数量、基线保护和影响传播。
- **Projection** 仅将可表示的确定性事实投影到当前 GuardFacts；集合逻辑不交给不存在的引擎算子。
- **Engine** 校验通用合同、评估中立规则、计算确定性证据，不解析 OpenSpec、不发批准。
- **Integration** 关联精确运行和外部审批；可信控制器核验身份、范围与新鲜度。

## 3. 领域模型与身份

下表均为目标内部模型，不是 `guard.partme.ai/v1alpha1` 可任意添加的字段。

| 对象 | 身份与含义 | 约束 |
|---|---|---|
| OriginalRequest | 原始诉求引用及摘要 | 保留来源，不凭生成文本替换 |
| Requirement | 项目命名空间 + 稳定需求 ID | 不以标题或行号作永久 ID |
| AcceptanceCriterion | 稳定验收 ID、所属需求、可观察条件 | 必要性由受保护策略/基线确定 |
| ArchitectureDecision | ADR 引用 | 存在引用不代表 ArchGuard 已验证 |
| Task | 实施任务引用及需求集合 | 一个任务可覆盖多个需求，必须显式声明 |
| TestObligation | 待证明的验收义务 | 与 TestGuard 的实际执行证据区分 |
| SourceRevision | 路径、候选/基线版本、字节摘要 | 行位置用于解释，摘要用于绑定 |
| ApprovalRef | 外部审批记录的不可变引用 | 需控制器认证，不信任布尔值 |

建议关系为 `refines`、`satisfies`、`verified_by`、`depends_on`、`supersedes`、`approved_from`。每种关系要有方向、允许的节点类型与基数；首期仅开放明确定义的关系。`approved_from` 仅表达已核验来源，不独立授予权限。`REQ-017 has_acceptance AC-017-1`、`TASK-024 implements REQ-017` 是领域关系示意，不是可直接提交引擎的完整 JSON。

ID 重命名必须有受审查的映射，检查双射冲突与映射链循环；没有映射时按删除/新增处理。文件移动且 ID 与内容摘要不变可归为位置变化。`depends_on` 循环是否允许必须由关系策略明确；不能一律拒绝所有图环，也不能无限遍历。

覆盖包括声明来源集合、观察来源集合、失败/忽略来源及原因、解析器支持范围。未知版本和被截断的文件不得算作已解析；过滤后的子集必须显式标为局部范围。

## 4. 批准基线与变更语义

基线至少绑定不可变 revision/digest、合同摘要、规格范围与外部审批引用。先认证基线，再比较候选；候选 Markdown 的 `approved: true`、勾选框或 “accepted” 不构成批准。批准者身份、角色、对象范围、有效期和撤销由受信控制器验证。

目标基线状态：`unregistered → proposed → approved → superseded`，批准也可能进入 `expired/revoked`。这是控制器及规格领域状态，不是引擎报告字段。SpecGuard 消费状态，不自行从 proposed 转为 approved。

| 差异类型 | 可确定部分 | 处置 |
|---|---|---|
| 新增 | 新 ID 或新增结构化条件 | 检查验收义务和关联范围 |
| 删除 | 基线 ID 在完整候选范围中不存在 | 无有效授权时产生确定性违例 |
| 移动/改名 | 相同 ID/摘要，或有效迁移映射 | 保留追踪，更新位置 |
| 引用重定向 | 相同源节点的关系目标变化 | 检查新目标并计算影响 |
| 结构化收紧/放宽 | 已知类型的比较运算及单位 | 类型规则明确时确定性分类 |
| 自由文本重述 | 内容摘要变化 | 默认语义待审，不自动判断等价/放宽 |

删除判断必须建立在完整范围上；解析失败导致“没看到”不等于被删除。只有预先声明的结构化类型才能比较强弱，例如明确单位和含义的延迟上限；不同单位或自然语言条件不得靠字符串比较推断放宽。

影响沿已定义关系反向索引传播，输出受影响需求、ADR、任务和测试义务以及传播路径。传播命中代表需复核，不代表下游测试已失败。基线升级必须生成新绑定；旧报告仍保留供审计，但不能复用于新候选。

## 5. 运行状态、决策与错误边界

目标阶段为 `discovered → snapshotted → parsed → normalized → compared → evaluated → emitted`。任一阶段可进入 `error` 或 `cancelled`。阶段完成不等于技术放行；只有输入有效、必要分析完成且引擎完成评估才有有效技术决策。

- 完整且无阻断结果：可能 `ALLOW`，范围仅限当前合同与分析器。
- 完整且存在 enforce 违例：`BLOCK`；review 命中可能产生 `REQUIRE_APPROVAL`。
- 能形成合法 partial GuardFacts：引擎给出 `BLOCK`/`INDETERMINATE`，不能由人工批准转换为完整。
- 输入无效、解析器崩溃且不能形成合法事实、验证失败：独立错误诊断，不伪造 GuardReport。目标 envelope 的 `runStatus=error`、`decision=null`。
- 取消：目标 envelope 的 `runStatus=cancelled`，不复用旧 ALLOW；中间产物仅作为标明不完整的诊断。

缺少配置的必需基线/审批核验能力是无法完成必要分析；已完整核验而确认未经授权的删除才是领域违例。将工具故障混成业务失败会掩盖重试路径，二者必须分开。

## 6. 协议与跨仓库合同

共享当前协议 `guard.partme.ai/v1alpha1` 保留历史标识，仅有 GuardContract YAML、GuardFacts JSON、GuardReport JSON；严格拒绝未知字段，规则只支持精确 `forbid_relation`，执行级别为 `enforce/review/advise`。该版本没有集合量化、任意字段或隐式插件扩展。详见上游 [Guard Protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)；此链接是兼容参考，不代表本仓库已执行其测试。

针对“每个需求至少有一个验收”，领域验证器先完整枚举必查需求，再计算缺口，最后生成合同可匹配的精确关系事实。不得从已通过子集反推必查集合。投影和合同生成必须经受保护配置控制；仅传入空事实而没有覆盖证明是不安全的设计。

GuardReport 无签名。verify 重算说明给定输入和结果是否一致，不证明事实来自可信解析器，也不赋予合并授权。

编排元数据使用独立的 [GuardRunEnvelope 草案](integration-contract.md)，版本 `guard.integration/v1alpha1`；当前引擎不解析此对象。它区分运行状态和可空决策，携带 repo/task/worktree/requirement/candidate/base/mergeGroup 绑定、contract/facts/report 摘要引用、分析器及覆盖、审批引用和诊断。领域图、源定位与审计附件独立保存，通过摘要引用；不能塞入旧 GuardReport。协议版本、crate semver 和策略 revision 各自独立。

| 集成方 | 目标交换 | 边界 |
|---|---|---|
| ArchGuard | 批准需求/ADR 引用及受影响项 | 不代替架构规则检查 |
| CodeGuard | 任务/需求映射与规格范围 | 不改动其既有 CLI/退出码；适配器须显式归一化 |
| TestGuard | 冻结验收义务与基线摘要 | 义务不等于成功测试证据 |
| GitGuard | 精确候选绑定、越权规格变更发现 | 不直接触发合并 |
| FlowGuard | 技术结果、审批引用、失效原因 | 控制器验证审批并组合门禁 |
| 外部规格工具 | 只读来源适配 | OpenSpec、Spec Kit、Superpowers、历史 spec-workflow-plugin 均未验证 |

六个独立守卫共享 GuardEngine；不引入 GuardCore。领域解析和政策保留在守卫中。

## 7. 证据失效、并发与审计

以下变化使受影响结果失效：candidate/base/merge group 改变、规则集或分析器/覆盖范围变化、批准基线 revision 变化、审批过期/撤销。分支名不够稳定，branch HEAD 的证据也不足以证明 merge queue 的候选；可信 CI 必须检查实际将合入的树。

幂等键以不可变运行绑定及输入摘要构成，不以需求 ID 单独作键。不同 task/worktree/requirement 集合隔离存储；旧任务晚返回时只能追加历史，不能覆盖当前候选指针。更新当前结果使用绑定比较或 compare-and-swap；相同绑定重试可去重。部分重跑只有覆盖与依赖闭包均证明有效时才能复用，首期默认完整重跑。

审计至少保留：来源清单与摘要、声明/实际覆盖、分析器版本、基线与合同摘要、发现到来源的映射、引擎输入/报告引用、外部审批引用、失效原因及运行时间。时间戳用于操作审计，不加入会破坏确定性重算的领域事实。默认避免输出需求全文和凭据，保留策略由部署者定义。

## 8. 安全与扩展

默认只读、无网络、不 fetch/install/init，不执行不可信文档、模板或策略脚本。仓库路径及符号链接必须限制在授权根；文件数、大小、AST 深度、图遍历量和解析时间要有预算，超限产生覆盖缺口/错误。只读凭据访问审批记录；任何未来写入接口都要独立授权。

候选可修改的规则和缓存均不可信。可信 CI 从受保护位置加载合同和固定解析器版本，独立生成事实，不能用 Agent 提交的“无违规”替代扫描。提示注入内容只作为待解析数据，不影响进程执行。

适配器扩展必须声明格式/版本、支持能力、无法解析范围与兼容 fixture；新关系需类型和循环策略；新增引擎算子必须走版本化协议变更，不能靠多写 YAML 字段启用。缓存为可删除的派生数据，不替代批准规格源。

## 9. ADR 与验收

- **SG-ADR-001**：既有规格源保留权威，图只作事实投影。
- **SG-ADR-002**：机器证明用于 enforce，业务理解用于 review。
- **SG-ADR-003**：检查前冻结必查集合，未知覆盖不可放行。
- **SG-ADR-004**：只读、显式范围，无隐式安装或初始化。
- **SG-ADR-005**：六守卫独立，编排归 FlowGuard，通用协议归 GuardEngine。
- **SG-ADR-006**：批准绑定不可变基线及外部认证记录。
- **SG-ADR-007**：编排 envelope 和领域附件与 v1alpha1 报告分离。
- **SG-ADR-008**：首期自由文本变化保守进入审查，不宣称确定性语义理解。

| 场景 | 目标验收证据 |
|---|---|
| 完整有效规格 | 精确范围和摘要相同的重复运行产生一致领域事实 |
| 重复 ID/断链/缺少强制验收 | 输出确定位置、关联规则与可重算 BLOCK |
| 无授权删除已批准验收 | 基线不变，保存删除来源及认证上下文 |
| 两个互斥权威源 | 明确冲突，不静默择一或 ALLOW |
| 文件损坏/不支持版本/资源超限 | partial 或 error，永不错误放行 |
| 候选伪造批准标识 | 不授予权限，真实审批引用核验可追踪 |
| 文本含义不明确 | REVIEW；不声称已证明收紧/等价 |
| 基线或候选变化 | 旧测试与规格证据失效，重新检查精确候选 |
| 并发任务晚返回 | 旧结果仅保留历史，当前指针不被覆盖 |

具体模块、错误映射和阶段准入见[技术设计](technical-design.md)。源版本支持矩阵、ID 迁移格式、审批提供方、摘要实现与资源预算仍是实施前需要决策的事项。
