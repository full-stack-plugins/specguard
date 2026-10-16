use serde::Deserialize;

/// 项目附加的单条字面内容义务；只证明文本存在，不证明语义执行。无 Java 来源。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextRequirement {
    pub id: String,
    pub text: String,
}
