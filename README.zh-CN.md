# SpecGuard — 规格守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**面向 AI 原生研发的需求、规格与验收基线守卫。**

> **当前状态：0.2.0 已实现 AGENTS.md 组织约定检查、原生 OpenSpec / Spec Kit 结构检查、产物 SHA256 漂移复验、CLI/MCP 及插件技能。** 业务语义、完整追踪图和受保护批准基线检查仍为规划能力。

运行 `specguard specs check --project 项目绝对目录 --source openspec|speckit` 检查既有原生结构；修复后重新 check，并用 `specs verify ... --baseline 绝对报告路径` 复验当前产物。详见 [范围、修复闭环和快照信任边界](docs/native-specs.md)。快照仅是未受信观察，不代表审批、实现或测试执行证据。

运行 `specguard specs sources --project 项目绝对目录`（MCP `specguard_spec_sources`）只读适配 OpenSpec / Spec Kit / Superpowers 来源并提取带作用域的稳定需求 ID。未知或多事实源冲突保持未决，Superpowers 计划本身不作为另一需求事实源。参见 [支持格式、ID 稳定边界和信任限制](docs/specification-sources.md)。该观察入口不能产生已批准基线。

运行 `specguard agents check --project 项目绝对目录` 检查六个 Guard 的提示注入和项目附加约定；`agents template` 返回审阅稿。参见 [使用及边界说明](docs/agents-context.md)、[插件](plugin/README.md)。AGENTS.md 官方不要求固定标题，六工具要求属于组织补充规范；文档 ALLOW 不表示智能体实际执行或审批通过。

## 解决什么问题

AI 能快速写出需求文档、开发任务与测试描述，但“生成出来了”不意味着需求被准确理解。SpecGuard 在已经批准的需求基线下，检查原始诉求、产品需求、验收、设计、编码任务和测试义务之间是否保持正确的结构与引用关系。

~~~text
原始诉求 → 需求规格 → 验收标准 → 设计/任务 → 测试义务
                   │
               追踪关系图
                   │
          结构规则 + 批准基线差异
                   │
         GuardEngine 规则与证据评估
                   │
           检查结果 → FlowGuard/CI
~~~

## 主要职责

- 只读接入 OpenSpec、Spec Kit、Superpowers、明确声明的 Markdown 事实源。
- 校验稳定 ID、追踪关系、缺失验收、弱化/删除批准条件、规格变更影响。
- 对可证明的违例使用 **ENFORCE**，对语义歧义使用 **REVIEW**，对优化建议使用 ADVISE。
- 保留来源、精确候选、合同版本与覆盖缺口；解析器失败时禁止误报通过。

SpecGuard 不负责产品业务批准（FlowGuard）、领域架构（ArchGuard）、真实测试执行（TestGuard）、Git 合并（GitGuard）。通用协议、规则执行和证据机制由 [GuardEngine](https://github.com/full-stack-plugins/guardengine) 提供。

## 文档

**[详细架构设计](docs/architecture.md)**：目标、组件、规格图、批准基线、与其他守卫的关系、信任边界、ADR、验收矩阵。

**[详细技术方案](docs/technical-design.md)**：Rust 技术选型、目录/数据模型、规则 DSL、CLI/MCP/CI、错误处理、分阶段任务和验证要求。

## 规划命令（尚不可执行）

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017
~~~

第一阶段建立可靠的只读事实提取与结构检查，后续增加批准基线差异、独立 CI 和多守卫协同。**绝不把自然语言理解或模型评分当成强制放行证据。**

## 智能体安装与提示注入

已接入共享安装器：`install`、`uninstall`、`context`、`instructions render|sync`、`doctor --integration`、`serve --mcp`。支持 Codex、Claude Code 和 Gemini CLI；按本工具职责生成独立标记区块。

```sh
cargo build --locked
specguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
specguard context --project /absolute/project
```

构建后请先通过 `target/debug/工具名`（Windows 加 `.exe`）调用；上面的短命令假定程序已加入 PATH。指定 `--bin-dir` 会复制当前二进制，安装器不自动修改 PATH；去掉 `--dry-run` 才实际安装。需要同级 `../guardengine/crates/guard-install` 源码。

[统一设计、宿主文件映射、更新、卸载和验证说明](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md)。安装成功不代表宿主已加载或专业检查通过。

需求到代码标记声明、测试执行证据关联已实现：见 [追踪协议](docs/trace-evidence.md)，不宣称语义正确或审批通过。
