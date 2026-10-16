---
name: specguard-native-specs
description: 检查既有 OpenSpec 或 Spec Kit 原生结构、需求标识与当前产物漂移，并完成检查、人工修复和复验闭环。
license: Apache-2.0
---

# 原生规格检查与摘要复验

1. 确认用户指定的绝对项目目录，读取项目指令、Git 状态及已有规格。用 `specguard spec context --project PROJECT` 只读识别体系，再依据当前需求确定唯一事实源。两体系并存时不自动选择，不初始化或安装新工具。
2. 首选 MCP `specguard_check_specs`，参数 `{"project":"绝对目录","source":"openspec"}` 或 `speckit`。CLI 为 `specguard specs check --project PROJECT --source openspec|speckit`。来源由项目事实决定，不为了获得 ALLOW 改选另一体系。
3. 阅读 decision、findings、diagnostics、requirements、sources 和 coverage。0/ALLOW 仅表示受支持结构检查通过；2/BLOCK 表示确定结构问题或摘要漂移；3/INDETERMINATE 表示覆盖不足；4 为参数或基线错误。工具不存在或失败时明确报告未检查。
4. 审阅 finding 并在授权范围内修复原始规格。保留需求、验收和非目标；核心变更遵守项目评审要求。不删规则、伪造场景或复制规格消除告警。tasks 勾选、文件存在、check ALLOW 都不能证明实现、测试执行或审批。
5. 修复后再次 check。将成功报告以 UTF-8 保存到被检查来源目录之外，例如 `.specguard/native-check.json`。快照只记录当前观察；已有快照不自动覆盖，也不能作为批准基线。
6. MCP `specguard_verify_specs` 参数增加 `"baseline":"绝对报告文件路径"`；CLI 为 `specguard specs verify --project PROJECT --source openspec|speckit --baseline ABSOLUTE_REPORT`。verify 重新读取来源，不信任旧报告的通过结论，对新增、删除、修改的产物逐项报告 baseline_drift。
7. 漂移后先审阅实际差异和相应评审证据，再重新 check；不得自动刷新报告使 verify 通过。记录两次报告、实际测试和未完成项。输出始终 `approval_status=not_evaluated`、`execution_status=not_evaluated`。

边界：当前仅支持 OpenSpec requirement/scenario 结构、Spec Kit FR 编号与字节摘要，不验证业务语义、需求到任务/测试的完整图、证据真实性或受保护批准基线。完整范围见仓库 `docs/native-specs.md`；GuardEngine 统一规格为 `professional-guard-checks`。
