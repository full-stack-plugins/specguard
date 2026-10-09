# SpecGuard — 技术实施方案

> 目标技术基线，尚未实现。检查日期：2026-10-09；检查 `main` SHA：`01137804aa465c1b931c73c9d211ff0d29b46c7a`。

## 1. 实现证据与选型边界

该 SHA 仅含 `README.md`、`README.zh-CN.md`、`docs/architecture.md`、本文件。没有 Rust/其他源码、包清单、测试、配置、schema、CI、OpenSpec 目录；没有运行时测试或 OpenSpec 验证可执行。本方案中的目录、类型、命令、错误名、性能预算和验收全部待实施。既有技术方向保留，但不将“写过设计”表述为能力已完成。

建议 Rust 2024、Serde、Clap、Markdown AST；JSON Schema 用于独立领域 DTO，Tokio 仅在异步 I/O 需要时引入。首期内存图与单 crate；SQLite 只作为后续可删除索引缓存，rmcp/GitHub Actions 为后续集成候选。版本固定与 SDK 接入方案需要实施时根据实际依赖确认。本仓库无依赖锁定或兼容测试。

领域解析、基线语义和审批引用处理属于 SpecGuard；GuardEngine 仅承担通用合同校验、规则执行和确定性证据计算。

## 2. 目标模块与适配器合同

以下是**规划目录**，不是当前文件结构：

~~~text
specguard/
├── Cargo.toml
├── src/{model,source,parser,graph,baseline,rules,evidence,cli}.rs
├── adapters/{openspec,speckit,superpowers,markdown}/
├── schemas/v1alpha1/          # SpecGuard 领域 DTO；不暗示引擎 schema 扩展
├── fixtures/{valid,duplicate,broken,baseline-change,partial}/
├── docs/
└── openspec/                 # 仅在正式引入设计流程后建立
~~~

| 模块 | 输入 → 输出 | 失败责任 |
|---|---|---|
| source | 授权根、显式来源、快照引用 → 文件清单/摘要 | 路径越界、来源冲突、读取中变化 |
| parser/adapters | 固定字节、格式/版本 → 节点、边、位置、覆盖 | 编码、语法、未知版本、未解析段 |
| graph | 节点/边 → 类型化索引、反向索引 | 重复身份、悬空引用、非法关系类型 |
| baseline | 认证基线 + 候选图 → 差异/影响闭包 | 基线缺失、审批不可核验、比较不确定 |
| rules | 冻结义务 + 图/差异 → 领域发现 | 覆盖不足不能声称未发现违规 |
| evidence | 领域发现、范围 → GuardFacts/附件引用 | 不支持的投影、摘要不一致 |
| cli | 命令/参数 → 稳定机器输出、退出码 | 错误与业务决策分离 |

目标适配器操作：`discover(root) → capabilities`、`parse(snapshot, declaredScope) → graphFragment + coverage + diagnostics`。这是内部接口伪代码，不是已提供的 API。能力声明至少说明 source kind、受支持版本、哪些节点/关系可提取、哪些正文结构不可分析。首期建议先实现严格、显式 ID 的 Markdown fixture，随后选择一个固定版本的 OpenSpec 格式；不把“能读取 Markdown”当成四种工具的兼容证明。

OpenSpec、Spec Kit、Superpowers、历史规格工作流插件都属于未验证目标。本次未检查外部插件或其旧包，不能宣称可直接复用。适配器不运行它们的安装器/脚本，不隐式生成配置。

## 3. 目标数据模型

字段表是 SpecGuard 内部 DTO 草案；精确 schema 和版本迁移将在实施时定义。不能将这些字段添加到严格的引擎 v1alpha1 对象。

| DTO | 建议字段 | 必须满足的约束 |
|---|---|---|
| SourceRef | rootId, path, lineStart, lineEnd, revision, digest | 相对授权根路径；摘要绑定实际字节；位置不能替代版本 |
| Requirement | id, namespace, origin, state, textDigest | 稳定且在命名空间唯一；state 不是批准凭证 |
| AcceptanceCriterion | id, requirementId, observable, expected, conditionType | 可选结构化类型需明确单位及比较语义 |
| TraceEdge | from, relation, to, source | 类型/方向合法；保留精确出处 |
| BaselineRef | approvedRevision, contentDigest, approvalRef, contractDigest | revision 不可变；审批由外部身份系统核验 |
| Coverage | requiredSources, observedSources, unparsedSources, analyzerScope | 每个必需源有终态；忽略原因不能冒充完成 |
| DomainFinding | ruleId, code, severity, nodeRefs, sourceRefs, detail | 无敏感全文；与引擎 decision 区分 |
| ImpactSet | changedIds, affectedIds, paths, baselineDigest | 传播路径可解释；不冒充测试失败 |

原始文件字节摘要用于来源验证；归一化图/事实使用固定排序与规范编码，避免目录顺序导致不确定结果。摘要算法和规范化规范待选，必须与引擎要求兼容，不能只凭字段名声称兼容。缓存键除字节摘要还包含解析器版本、来源版本、配置/合同摘要及覆盖范围。

ID 不从行号生成。首期不自动猜测重命名，默认删除/新增；显式映射需要唯一性、无环校验和批准范围。失效的源映射不能静默改写为新的来源。

## 4. 处理流程和核心算法

1. **discover**：读取显式配置；枚举允许根中的来源及能力，报告冲突，不创建文件。
2. **snapshot**：固定 candidate/base/merge group、来源清单、必查 ID、合同和批准基线；dirty 工作区需固定内容快照，不能只记录 HEAD。
3. **parse**：对快照字节解析，逐源记录完成/失败/忽略原因；未知版本、截断、坏 YAML 和编码错误不可吞掉。
4. **normalize**：按 namespace/ID 建索引；保留重复项定位，不用“最后一个覆盖前一个”。校验关系类型及目标存在性。
5. **diff**：对完整范围按稳定 ID 比较基线和候选；有合法迁移映射才重识别身份。部分范围不推断删除。
6. **impact**：用反向邻接表遍历受影响义务，visited 集合防止环，记录传播理由；按策略决定遍历哪些关系。
7. **validate**：先验证覆盖，再执行必查集合的结构规则；自然语言变更进入 review。结构化弱化比较仅对已声明类型运行。
8. **project/evaluate**：构造严格兼容的事实和合同输入，交给引擎；不支持投影时明确报错，不丢弃检查项。
9. **emit/recheck**：保存完整输出或标明不完整的附件；可信 CI 从受保护策略重新分析精确合入候选。

图索引与单次遍历目标为 O(V+E)，确定性输出排序另需 O(V log V + E log E)；复杂文本 diff 的成本须单独限制。不得为输出所有可能传播路径造成指数爆炸；默认每个受影响节点保存一个确定性代表路径及计数，完整路径作为可限额扩展。

### 4.1 规则与当前引擎的衔接

当前共享协议仅支持 `forbid_relation` 精确匹配，非通用图查询。原有 DSL 方向保留，但以下用**领域伪代码**而非未验证的完整 YAML 展示，避免将目标字段误认作可运行协议：

~~~text
protected baseline requires AC-017
candidate scope is complete
trusted approval lookup confirms no deletion authorization
  => emit exact relation:
     subject = AC-017
     predicate = removed_without_approval
     object = approved-baseline
protected rule forbids that exact triple with enforcement = enforce
~~~

将来完整协议 fixture 必须从引擎实际 schema/示例生成并验证；此处不声称示例已经通过解析。`REQ-017 has_acceptance AC-017-1` 和 `TASK-024 implements REQ-017` 也是概念关系，不是完整 GuardFacts。

“每个需求至少一个验收”由领域验证器完整遍历冻结集合，计算具体缺口；禁止用不存在的 `count`/`forall`/`extensions` 字段假装引擎支持。精确规则覆盖与事实投影需一并验证，尤其要证明所有强制缺口都有规则匹配；不允许遗漏未映射 finding 后返回 ALLOW。未建立可靠投影前，该检查只能标记未支持，不得用于可信门禁。

业务 review 的请求可以保存在独立领域附件；需要引擎 `REQUIRE_APPROVAL` 时必须使用真实可表示的受保护规则。自然语言检查建议可以附带模型来源，但不能自动升级为确定性 enforce。

## 5. 外部接口草案

以下命令**全部不存在、不可执行**：

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017 --format json
specguard diff --base <approved-ref> --head HEAD
specguard check --project . --format json
~~~

| 命令 | 目标职责/输出 | 必需前提 |
|---|---|---|
| doctor | 来源、版本与缺失能力诊断 | 不自动修复/安装；探测不算通过门禁 |
| scan | 规格图、覆盖、来源清单/摘要 | 显式范围和快照；不默认为批准检查 |
| trace | 某需求到验收/任务/测试义务路径 | 索引与当前快照一致，不存在的 ID 可诊断 |
| diff | 认证基线与候选的差异/影响 | base 必须解析成不可变、已核验基线 |
| check | 完整管线 + 引擎评估 + 集成结果 | 受保护合同、可信分析器、完整必需范围 |

`<approved-ref>` 是占位符，不是可直接复制执行的 shell 参数。`--head HEAD` 仅表达本地候选选择；可信 CI 需要绑定实际 merge-queue candidate，而非固定采用分支 HEAD。选取精确候选和基线配置的最终参数仍需定义。

目标 `check` 退出合同：`0` ALLOW，`2` BLOCK，`3` REQUIRE_APPROVAL，`4` 输入/运行/验证错误。合法 partial 事实被引擎评估为 BLOCK 时退出 2；无法形成有效运行结果的工具失败退出 4。取消的传输/退出行为需按 CLI、MCP、CI 分别定义，不得约定为成功。其他查询命令退出合同待稳定；不可把该表说成现有 CLI 行为。

目标 `--format json` 输出一个完整 JSON 对象到 stdout；日志/进度/诊断到 stderr，避免混合输出。没有已实现或确定语义的 `--report` 参数。GuardReport 是严格引擎对象；独立错误对象和领域附件不能冒充其格式。

MCP 首期仅开放发现、检查与证据查询；精确工具名、认证和取消合同待定义。不开放提交、修改规格、批准或合并。CI 使用受保护合同与固定分析器，保存内容寻址附件及候选绑定；SARIF 是后续定位投影，不取代引擎报告和覆盖证明。

编排使用 [GuardRunEnvelope 草案](integration-contract.md)（`guard.integration/v1alpha1`），当前引擎不解析。`runStatus` 为 completed/error/cancelled；失败时 decision 可空。绑定包括 repoId/taskId/worktreeId/requirementIds/candidateOid/baseOid/mergeGroupId，另引用 contract/facts/report 摘要、分析器/覆盖、审批和诊断。不得把 envelope 混入 `guard.partme.ai/v1alpha1`。版本与 crate semver、策略 revision 分开演进。

## 6. 错误分类、恢复与状态

以下错误码为目标诊断名称，不是已实现的枚举或引擎 schema：

| 类别 | 解释 | 目标处置/恢复 |
|---|---|---|
| InvalidSource | 语法、编码或不支持格式 | 标明文件/位置；partial 或 error；修复后重跑 |
| DuplicateIdentity | 已完整识别的身份冲突 | 领域发现，enforce 可 BLOCK；修正 ID/映射 |
| MissingReference | 已完整检查的目标不存在 | 领域发现；不能用解析失败伪造此结论 |
| AmbiguousAuthority | 多个互斥来源 | 必要分析不完整；显式选定权威源后重跑 |
| BaselineDrift | 基线或批准范围与绑定不符 | 旧证据失效，重新固定基线并检查 |
| ApprovalUnverified | 无法核验必要审批 | 不发批准；恢复认证依赖后重跑 |
| AnalyzerPartial | 解析范围不足/预算截断 | 合法 partial → BLOCK/INDETERMINATE |
| ContractUnsupported | 合同无效或不支持的算子 | error，不能降低为建议后继续放行 |
| SnapshotChanged | 读取时内容改变 | 丢弃混合结果，重新冻结快照 |
| VerificationMismatch | 输入/输出摘要或重算不符 | error，保留诊断并重新分析 |

运行阶段状态见[架构](architecture.md)。成功解析不等于成功检查；有完整确定性违例可形成 completed/BLOCK，而 error 不应有伪造业务决策。审批能处理有效范围内的待审项，不能覆盖错误、超时、partial 或未认证基线。

仅对明确瞬态读取失败执行有界重试；不自动重试以消除确定性违规。重试生成新 run 标识并保留同一不可变输入键。输出写入建议采用临时文件 + 原子重命名，错误/取消不留下看似完整的报告。

## 7. 证据、并发、缓存与安全

首期默认无持久缓存，以完整快照重跑确保一致性。后续缓存键至少包含项目/来源命名空间、输入内容摘要、分析器及版本、合同摘要、覆盖配置、基线摘要；审批新鲜度应重新核验，不能仅靠缓存命中。

集成结果还必须绑定 task/worktree/requirement 集合、candidate/base/merge group。候选、base、merge group、规则集、分析器/覆盖、基线 revision、审批有效期/撤销任一变化，都使受影响结果失效。不同任务不共享可覆盖的“latest.json”；当前指针更新要比较完整绑定，旧结果晚返回只追加历史。

审计附件保存来源位置/摘要、必查与实际覆盖、差异/影响路径、规则和分析器版本、基线/审批引用、事实/报告摘要及失效原因。引擎报告无签名，verify 仅重算；可信 CI 还需要认证执行环境和外部授权。ALLOW 只是限定范围的技术结论，不能直接触发合并或发布。

安全默认：只读文件/凭据；本地无网络模式；审批读取通过显式可信适配器；不执行候选中的脚本/指令；固定解析器版本；合同由受保护位置提供。限制符号链接、绝对路径、`..` 跳转、跨根引用。检查 Git 快照时也不能把仓库中链接目标当成授权路径。

实施时必须给出并测试文件数、总字节数、单文件大小、AST 深度、图节点/边数量、超时预算。具体数值尚未测量，不能宣传性能 SLA。默认日志不含需求全文、token 或凭据，附件保留期限与加密由部署策略定义。

## 8. 分阶段交付与可测量准入

所有阶段为未来工作；本次没有完成其中的运行能力。

| 阶段 | 交付 | 可测量准入证据 |
|---|---|---|
| S0 | doctor、快照、SourceRef、能力声明 | 有源/无源/冲突/越界四类 fixture；运行前后目录内容无变化；每个必需源都有状态 |
| S1 | 一个固定版本适配器、图、结构规则、引擎投影 | 每条 enforce 规则四类 fixture 全覆盖；重复 ID 不丢失；乱序输入重复运行事实一致；未映射 finding 不可放行 |
| S2 | 认证基线、diff、影响闭包 | 删除/伪批准/审批失效/自由文本变化/合法移动各有正反例；partial 不误判删除；循环遍历有限结束 |
| S3 | 跨守卫 envelope、精确候选绑定 | candidate/base/merge group/规则/分析器/覆盖/基线/审批八类失效测试；并发任务隔离及晚结果不覆盖当前结果 |
| S4 | 稳定 CLI、MCP/CI、缓存与兼容矩阵 | 退出码和 stdout/stderr 契约测试；资源超限/取消测试；缓存开关结果一致；已声明 N/N-1 版本 fixture；真实仓库只读演练 |

每条硬约束的四类 fixture 是：合法、确定违规、工具故障、覆盖不足。每个适配器必须有未知版本和混合格式样本。引擎协议 fixture 需实际验证未知字段拒绝、只允许当前算子、partial 不放行及报告重算；不能把文档中的示意算作通过。

S3 集成验收需使用可核验的对端版本和真实附件；在对端未实现时仅能标为模拟合同测试。N/N-1 仅针对将来明确发布的版本，不宣称当前已有两代兼容。退出码适配需保留 CodeGuard 原有 CLI 行为，并在适配层显式转换。

实施报告应记录确切命令、版本、fixture 数量、失败项和运行产物；只有真实结果才能标注“已验证”。本次文档验证限于差异、相对链接、双语范围、命名、目标/实现区分；无源码修改、运行时测试或 OpenSpec 验证。

## 9. 待决事项与可逆默认值

| 待决事项 | 建议可逆默认值 | 决策前约束 |
|---|---|---|
| 来源版本/配置语法 | 一个明确版本 + 显式根/ID | 不自动检测后猜测格式 |
| ID 迁移表示 | 不猜重命名；后续受审查映射 | 删除/新增需要真实基线核验 |
| 结构化条件类型 | 首期只做存在性/引用；文本改动 review | 不宣称比较自然语言强弱 |
| 审批提供方 | 只读认证接口，可在测试中注入 | 生产不得信任测试批准或布尔值 |
| hash/规范编码/SDK 版本 | 按引擎实际实现固定并做 fixture | 不发明兼容编码或扩展字段 |
| 性能及资源预算 | 首期无缓存，有界全量扫描 | 超限失败关闭门禁，测量后再调优 |
| CI/MCP 部署方式 | 先稳定本地只读接口 | 不引入默认副作用或合并权限 |

这些选择需要实现验证或维护者决策，不妨碍当前文档作为待实施基线。后续扩展引擎协议必须版本化并有迁移策略，不能悄悄改变当前 v1alpha1 的严格字段约束。


### 集成错误的绑定前置条件

上述目标 error/cancelled 信封只适用于调用身份、精确候选/基底、producer 与必查覆盖已经冻结的尝试。参数非法、仓库不可解析或绑定歧义等前置故障使用独立传输诊断和失败退出状态，不生成 GuardRunEnvelope，不伪造 OID 或空字段；当前各 CLI 的既有行为仍按本文事实表保留。详见[共享契约](integration-contract.md)。
