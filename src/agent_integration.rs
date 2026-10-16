use guard_install::DomainTool;
use serde_json::{Value, json};

const DOMAIN_TOOLS: &[DomainTool] = &[
    DomainTool {
        name: "specguard_check_trace",
        description: "Read-only declared native requirement/source/test linkage; PASS requires current signed execution and explicit external trust anchor. No semantic or approval claim.",
        schema: trace_schema,
        call: check_trace,
    },
    DomainTool {
        name: "specguard_verify_trace",
        description: "Read-only full trace recomputation against a prior report; rejects drift or forged results.",
        schema: trace_verify_schema,
        call: verify_trace,
    },
    DomainTool {
        name: "specguard_check_specs",
        description: "Read-only native OpenSpec/Spec Kit structure and artifact digest check. Explicit source required; no execution or approval evidence.",
        schema: specs_schema,
        call: check_specs,
    },
    DomainTool {
        name: "specguard_verify_specs",
        description: "Read-only recheck against a previous check report; reports any observed artifact drift. Baseline is untrusted observation, never approval.",
        schema: verify_schema,
        call: verify_specs,
    },
    DomainTool {
        name: "specguard_check_agents",
        description: "Read-only check of root AGENTS.md against six-guard-context/v1 and additive project text policy. Does not prove runtime execution or approval.",
        schema,
        call: check,
    },
    DomainTool {
        name: "specguard_agents_template",
        description: "Return a six-Guard instruction template for review, without writing files or installing tools.",
        schema,
        call: template,
    },
];

/// 返回真实接入能力；安装、上下文查询和文档检查均不替代专业执行或审批。
pub(crate) fn tool() -> guard_install::Tool {
    let mut tool = guard_install::Tool::new(
        "specguard",
        env!("CARGO_PKG_VERSION"),
        "开始需求、设计或实现前运行 agents check 检查根 AGENTS.md 的 six-guard-context/v1 组织约定与 .specguard/agents-policy.json 附加义务；用 agents template 生成只读审阅稿，修复后重新 check。用 spec context 确认唯一原生规格，不重复初始化第二套体系。运行 specs check --project ABS --source openspec|speckit 检查原生结构并记录产物SHA256；修复后重新check，再用 specs verify --baseline ABS_REPORT 复验当前产物是否漂移。快照不是批准基线，不自动接受新基线。核心需求、非目标和验收条件变更需相应评审。文档合规不代表智能体实际执行、安装或审批通过；用 trace check/verify 核对显式需求、代码marker及真实测试证据关联；必须显式manifest、外部信任公钥和当前签名执行证据才能PASS。关联不证明业务语义。",
        &[
            "agents check",
            "agents template",
            "specs check",
            "specs verify",
            "trace check",
            "trace verify",
        ],
    );
    tool.mcp_tools = DOMAIN_TOOLS;
    tool
}

fn schema() -> Value {
    json!({"type":"object","properties":{"project":{"type":"string"}},"required":["project"],"additionalProperties":false})
}
fn invoke(args: &Value, action: &str) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    if object.len() != 1 {
        return Err("only project is accepted".into());
    }
    let project = object
        .get("project")
        .and_then(Value::as_str)
        .ok_or("project string required")?;
    crate::agents_cli::run(&[
        "agents".into(),
        action.into(),
        "--project".into(),
        project.into(),
    ])
    .map(|(value, _)| value)
}
fn check(args: &Value) -> Result<Value, String> {
    invoke(args, "check")
}
fn template(args: &Value) -> Result<Value, String> {
    invoke(args, "template")
}

fn specs_schema() -> Value {
    json!({"type":"object","properties":{"project":{"type":"string"},"source":{"type":"string","enum":["openspec","speckit"]}},"required":["project","source"],"additionalProperties":false})
}
fn verify_schema() -> Value {
    let mut schema = specs_schema();
    schema["properties"]["baseline"] = json!({"type":"string"});
    schema["required"] = json!(["project", "source", "baseline"]);
    schema
}
fn invoke_specs(args: &Value, verify: bool) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    if object.len() != if verify { 3 } else { 2 }
        || object
            .keys()
            .any(|k| !["project", "source", "baseline"].contains(&k.as_str()))
    {
        return Err("only project/source and verify baseline are accepted".into());
    }
    let mut cli = vec![
        "specs".into(),
        if verify { "verify" } else { "check" }.into(),
        "--project".into(),
        object
            .get("project")
            .and_then(Value::as_str)
            .ok_or("project string required")?
            .into(),
        "--source".into(),
        object
            .get("source")
            .and_then(Value::as_str)
            .ok_or("source string required")?
            .into(),
    ];
    if verify {
        cli.extend([
            "--baseline".into(),
            object
                .get("baseline")
                .and_then(Value::as_str)
                .ok_or("baseline string required")?
                .into(),
        ]);
    }
    crate::specs_cli::run(&cli).map(|(value, _)| value)
}
fn check_specs(args: &Value) -> Result<Value, String> {
    invoke_specs(args, false)
}
fn verify_specs(args: &Value) -> Result<Value, String> {
    invoke_specs(args, true)
}

fn trace_schema() -> Value {
    json!({"type":"object","properties":{"project":{"type":"string"},"manifest":{"type":"string"},"evidence":{"type":"string"},"public_key":{"type":"string"}},"required":["project","manifest"],"additionalProperties":false})
}
fn trace_verify_schema() -> Value {
    let mut schema = trace_schema();
    schema["properties"]["report"] = json!({"type":"string"});
    schema["required"] = json!(["project", "manifest", "report"]);
    schema
}
fn invoke_trace(args: &Value, verify: bool) -> Result<Value, String> {
    let object = args.as_object().ok_or("object required")?;
    let mut cli = vec![
        "trace".into(),
        if verify { "verify" } else { "check" }.into(),
    ];
    for (key, value) in object {
        if !["project", "manifest", "evidence", "public_key", "report"].contains(&key.as_str())
            || (!verify && key == "report")
        {
            return Err("unknown trace argument".into());
        }
        cli.push(format!("--{}", key.replace('_', "-")));
        cli.push(
            value
                .as_str()
                .ok_or("trace argument must be string")?
                .into(),
        );
    }
    crate::trace_cli::run(&cli).map(|(v, _)| v)
}
fn check_trace(args: &Value) -> Result<Value, String> {
    invoke_trace(args, false)
}
fn verify_trace(args: &Value) -> Result<Value, String> {
    invoke_trace(args, true)
}
