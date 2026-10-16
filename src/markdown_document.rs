use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::BTreeMap;

/// CommonMark 正文和独立行标记的只读投影；排除代码、引用和注释示例。
pub(crate) struct MarkdownDocument {
    pub lines: BTreeMap<usize, String>,
    pub markers: Vec<(usize, String)>,
}
impl MarkdownDocument {
    /// 将输入 Markdown 投影为带行号的正文与标记；不执行或解释其中的命令。
    pub(crate) fn parse(source: &str) -> Self {
        let mut result = Self {
            lines: BTreeMap::new(),
            markers: Vec::new(),
        };
        let mut offsets = vec![0];
        offsets.extend(source.match_indices('\n').map(|(index, _)| index + 1));
        let source_lines: Vec<_> = source.lines().collect();
        let mut excluded = 0usize;
        let mut standalone_comments = false;
        for (event, range) in Parser::new(source).into_offset_iter() {
            let line = offsets.partition_point(|offset| *offset <= range.start);
            match event {
                Event::Start(Tag::HtmlBlock) => {
                    // Html 事件可能逐行返回同一个多行注释，必须检查整个块的词法边界。
                    standalone_comments = source[range].lines().all(|line| {
                        let line = line.trim();
                        line.is_empty()
                            || (line.starts_with("<!-- ")
                                && line.ends_with(" -->")
                                && line.matches("<!--").count() == 1
                                && line.matches("-->").count() == 1)
                    });
                }
                Event::End(TagEnd::HtmlBlock) => standalone_comments = false,
                Event::Start(Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::Image { .. }) => {
                    excluded += 1
                }
                Event::End(TagEnd::CodeBlock | TagEnd::BlockQuote(_) | TagEnd::Image) => {
                    excluded -= 1
                }
                Event::Text(value) | Event::Code(value) if excluded == 0 => {
                    for (delta, text) in value.split('\n').enumerate() {
                        result.lines.entry(line + delta).or_default().push_str(text);
                    }
                }
                Event::Html(value) if excluded == 0 && standalone_comments => {
                    let text = value.trim();
                    if text.starts_with("<!-- ")
                        && text.ends_with(" -->")
                        && source_lines
                            .get(line - 1)
                            .is_some_and(|s| s.trim().trim_start_matches('\u{feff}') == text)
                    {
                        result.markers.push((line, text.into()));
                    }
                }
                _ => {}
            }
        }
        result
    }
}
