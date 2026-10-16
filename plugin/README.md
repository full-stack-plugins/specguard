# SpecGuard plugin

Agent Plugins 1.0.0 可移植包；包含 specguard-agents-context 技能及 stdio MCP 配置。依赖可从宿主 PATH 找到的 SpecGuard 0.2.0 或更新兼容版本；包不自动安装二进制。

新增 `specguard-native-specs` 技能及 MCP `specguard_check_specs`、`specguard_verify_specs`：显式选择原生 OpenSpec/Spec Kit，检查结构、输出文件摘要，再以成功观察报告复验漂移。CLI 为 `specs check` / `specs verify`，详见 [范围与信任边界](../docs/native-specs.md)。不把任务勾选或文件存在当作实现证据；快照不等于批准基线。

MCP：specguard_context、specguard_instructions、specguard_check_agents、specguard_agents_template。检查/模板均只读。CLI 同源入口为 agents check / agents template。

技能提供检查 → 审阅修复 → 已授权同步 → 当前文件复验流程。安装提示与规范检查由不同职责执行，不产生审批权限。标准包布局、MCP 运行和宿主加载需要分别验证，不能以包存在宣称宿主已加载。
