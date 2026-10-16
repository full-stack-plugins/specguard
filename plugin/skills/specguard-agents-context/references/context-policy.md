# 上下文约定 v1

官方基线：https://agents.md/ ，2026-10-16 核对。该标准没有强制字段；嵌套文档和指令冲突应按宿主及上级指令处理。

组织 profile `six-guard-context/v1` 要求明确项目根中存在 AGENTS.md，六个闭合且唯一的块：ARCHGUARD、CODEGUARD、FLOWGUARD、GITGUARD、SPECGUARD、TESTGUARD，使用 `<!-- NAME_START -->` / `<!-- NAME_END -->`。

每块使用共享安装器生成的触发、入口、规则、失败处理和证据五条正文；不得以代码块、引用或注释中的示例冒充。模板内容采用固定字面匹配，不做自然语言语义等价判断；需要自定义说明时保留这些条目并添加项目说明。

可选 `.specguard/agents-policy.json`：

```json
{
  "schema_version": 1,
  "required_text": [
    {"id": "single-spec-source", "text": "同一变更只保留一个规格事实源。"},
    {"id": "preserve-worktree", "text": "不得覆盖用户未提交的修改。"}
  ]
}
```

附加 text 在可见正文中按字面匹配，不能关闭默认六块。文件每个最多 1 MiB，最多 128 项，每条 text 最多 4096 字节；UTF-8、非链接普通文件是本分析器限制，不能解释为官方禁止符号链接。

策略位于工作区，可能被修改；报告摘要不是签名或审批。受保护 CI 需要固定可信策略及候选代码版本。检查不执行文档中的命令，不读取宿主实际加载状态，也不验证全文不存在冲突或恶意指令。
