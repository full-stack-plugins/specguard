## ADDED Requirements

### Requirement: Read-only versioned source discovery
SpecGuard SHALL 仅发现显式授权根中的规格来源，声明支持的格式/版本和冲突，并 MUST 禁止隐式安装、初始化、修改源文件或执行文档指令。

#### Scenario: Supported explicit source
- **WHEN** 显式根中存在支持版本的规格
- **THEN** 发现结果包含版本和必查文件清单，运行前后源文件字节相同。

#### Scenario: Ambiguous or unsupported authority
- **WHEN** 存在冲突权威源或不支持版本
- **THEN** 结果标记无法完成必要分析，不自行择一并报告完整。

### Requirement: Frozen source scope and bounded parsing
SpecGuard SHALL 在解析前冻结必查来源和内容快照，记录逐源覆盖、字节摘要及来源位置，并 MUST 对路径逃逸、资源超限和读取漂移失败关闭。

#### Scenario: Partial source
- **WHEN** 一个必需文件损坏或超出已配置预算
- **THEN** 覆盖包含该文件及原因，不产生完整性通过结论。

#### Scenario: Snapshot or path drift
- **WHEN** 读取中内容改变或符号链接越出授权根
- **THEN** 拒绝混合/越界输入并给出可定位诊断。

### Requirement: Stable requirement identity and typed trace graph
SpecGuard SHALL 以命名空间和稳定 ID 构建类型化追踪图，保留重复身份定位，校验关系方向/目标，并 MUST 仅通过显式经审查的无环唯一映射识别重命名。

#### Scenario: Duplicate and broken reference
- **WHEN** 输入有重复 ID 或悬空关系且相关范围完整
- **THEN** 输出每个冲突来源或缺失目标，不用最后一个节点覆盖冲突。

#### Scenario: Reordered and renamed input
- **WHEN** 等价输入顺序改变，或文件移动但稳定 ID 不变
- **THEN** 图事实保持确定；没有有效映射的身份变化按删除/新增处理。

### Requirement: Frozen deterministic acceptance obligations
SpecGuard SHALL 在检查前冻结必查需求/验收集合，对每个必查需求执行结构规则，并 MUST 不从成功子集反推完整范围。

#### Scenario: Missing mandatory acceptance
- **WHEN** 完整需求范围内某必需需求没有验收链接
- **THEN** 生成可精确定位的领域违例。

#### Scenario: Unparsed target
- **WHEN** 验收链接目标所在文件未解析
- **THEN** 报告覆盖不足，不把未知状态伪装成已确定删除或不存在。

### Requirement: Authenticated immutable approved baseline
SpecGuard SHALL 消费绑定仓库/范围、不可变 revision/content digest、policy digest、有效期及外部审批引用的基线，MUST 由可信控制器核验身份、范围和撤销状态，不自行发放批准。

#### Scenario: Forged approval label
- **WHEN** 候选 Markdown 声称 approved 或 accepted 而无可认证审批
- **THEN** 该标签不授予批准，必要核验不能完成时不放行。

#### Scenario: Expired or revoked approval
- **WHEN** 已记录审批过期或被撤销
- **THEN** 受影响基线证据失效并要求重新认证/分析。

### Requirement: Conservative baseline diff and impact
SpecGuard SHALL 在完整可比较范围按稳定 ID 计算新增/删除/移动/引用变化，仅对已审核结构化类型判定强弱，MUST 将自由文本含义变化送审并以有界图遍历解释影响。

#### Scenario: Unauthorized removal
- **WHEN** 完整候选删除批准验收且真实核验确认无授权
- **THEN** 产生 enforce 候选发现并保留基线与来源，不修改批准基线。

#### Scenario: Semantic text or cyclic impact
- **WHEN** 自由文本发生变化且影响图含允许的环
- **THEN** 文本变化进入 review，影响遍历终止并输出确定性解释路径，不声称行为测试失败。

### Requirement: Versioned frozen TestObligation export
SpecGuard SHALL 导出独立版本化的 TestObligation 附件，包含稳定义务/需求/验收 ID、来源/基线/候选摘要、必查范围与覆盖，MUST 不将义务存在表述为测试成功。

#### Scenario: Complete obligation handoff
- **WHEN** 已认证基线与完整范围生成验收义务
- **THEN** TestGuard 可按稳定 ID 和摘要读取冻结义务，附件不包含未经执行的成功证明。

#### Scenario: Incomplete handoff
- **WHEN** 规格范围 partial
- **THEN** 输出明确不完整状态，消费者不得将其作为完整冻结验收计划。
