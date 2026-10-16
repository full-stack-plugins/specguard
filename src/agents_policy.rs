use crate::text_requirement::TextRequirement;
use serde::Deserialize;
use std::collections::HashSet;

/// 在组织默认六工具约定上增加项目文本义务的配置；不提供关闭默认规则的开关。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentsPolicy {
    pub schema_version: u32,
    pub required_text: Vec<TextRequirement>,
}
impl AgentsPolicy {
    /// 解析输入 JSON 字节并校验版本、数量和唯一 ID；返回有效策略或诊断。
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        let policy: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if policy.schema_version != 1 || policy.required_text.len() > 128 {
            return Err("unsupported policy version or requirement count exceeds 128".into());
        }
        let mut ids = HashSet::new();
        for rule in &policy.required_text {
            if rule.id.trim().is_empty()
                || rule.id.len() > 128
                || rule.text.trim().is_empty()
                || rule.text.len() > 4096
                || !ids.insert(&rule.id)
            {
                return Err("empty/oversized requirement or duplicate rule id".into());
            }
        }
        Ok(policy)
    }
}
