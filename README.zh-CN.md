# Partme SpecGuard — 规格守卫

[English](README.md) · [简体中文](README.zh-CN.md)

**面向 AI 原生研发的需求、规格与验收基线守卫。**

> **当前状态：已完成详细架构与技术方案文档，尚未实现可执行的 SpecGuard CLI、MCP 或 CI 门禁。** 下方命令只是目标接口。

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
