---
name: specguard-trace
description: 只读核验原生需求、代码标记声明和受信执行测试的追踪闭环。
license: Apache-2.0
---

1. 读取项目指令并确定唯一原生规格来源。不初始化工具，不改选来源绕过缺失需求。
2. 准备显式 specguard.trace/v1 manifest，逐条绑定 artifact + requirement_id、项目内代码路径 + marker 和真实测试 ID；execution_plan_digest 必须来自冻结 TestGuard 计划。
3. 运行 `specguard trace check --project ABS --manifest ABS --evidence ABS --public-key ABS`，或只读 MCP `specguard_check_trace`。公钥必须来自项目外显式受信配置，不从回执自选，不生成替代密钥。
4. 无证据只可 INDETERMINATE。缺映射、空测试、无效引用、过期源码或假执行证据必须阻断。不能通过删需求、降低测试集合或覆盖报告消除失败。
5. 在授权范围内修复，再真实执行 TestGuard 计划，产生新的签名回执并重跑 check。报告和执行输出保存在项目外。
6. 运行 `trace verify`，同参数加 `--report ABS`；MCP `specguard_verify_trace` 同义。完整复算当前数据，不把旧PASS视为审批。
7. 报告真实命令、结果和剩余限制。marker与执行证据关联不等于语义正确，semantic_validation / approval_status 始终 NOT_EVALUATED。协议详见仓库 docs/trace-evidence.md。
