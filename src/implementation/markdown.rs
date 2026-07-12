use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

pub fn render_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    let mut regular_events = Vec::new();
    let mut events = parser.into_iter();

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                push_regular_html(&mut html_output, &mut regular_events);

                let mut source = String::new();
                for next_event in events.by_ref() {
                    match next_event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) | Event::Code(text) | Event::Html(text) => {
                            source.push_str(&text);
                        }
                        Event::SoftBreak | Event::HardBreak => source.push('\n'),
                        _ => {}
                    }
                }

                html_output.push_str(&render_code_block(
                    code_block_language(&kind),
                    &normalize_code_block_source(&source),
                ));
            }
            other => regular_events.push(other),
        }
    }

    push_regular_html(&mut html_output, &mut regular_events);
    html_output
}

pub fn extract_title(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return None;
        }

        let title = trimmed.trim_start_matches('#').trim();
        if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        }
    })
}

fn push_regular_html<'a>(html_output: &mut String, events: &mut Vec<Event<'a>>) {
    if events.is_empty() {
        return;
    }

    let buffered = std::mem::take(events);
    html::push_html(html_output, buffered.into_iter());
}

fn code_block_language(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Indented => None,
        CodeBlockKind::Fenced(info) => {
            let token = info
                .split(|character: char| character.is_whitespace() || character == ',')
                .find(|part| !part.is_empty())?;
            Some(token.to_ascii_lowercase())
        }
    }
}

fn render_code_block(language: Option<String>, source: &str) -> String {
    if matches!(language.as_deref(), Some("mermaid")) {
        return render_mermaid_block(source);
    }

    if matches!(language.as_deref(), Some("math")) {
        return render_math_block(source);
    }

    let highlight = highlight_code(language.as_deref(), source);
    let badge = highlight
        .language_label
        .as_ref()
        .map_or_else(String::new, |label| {
            format!(
                "<div class=\"code-block__badge\">{}</div>",
                escape_html(label)
            )
        });
    let language_attribute = highlight
        .language_label
        .as_ref()
        .map_or_else(String::new, |label| {
            format!(" data-language=\"{}\"", escape_html_attribute(label))
        });
    let line_numbers = render_line_numbers(highlight.line_count);

    format!(
        "<div class=\"code-block\"{language_attribute}>{badge}<div class=\"code-block__body\"><div class=\"code-block__line-numbers\" aria-hidden=\"true\">{line_numbers}</div><pre class=\"code-block__pre\"><code class=\"code-block__code\">{}</code></pre></div></div>",
        highlight.lines_html.join("")
    )
}

fn render_mermaid_block(source: &str) -> String {
    format!(
        "<div class=\"mermaid-block\"><div class=\"mermaid-block__status\">Rendering diagram...</div><pre class=\"mermaid-block__source hidden\">{}</pre><div class=\"mermaid-block__diagram\"></div></div>",
        escape_html(source)
    )
}

fn render_math_block(source: &str) -> String {
    format!(
        "<div class=\"math-block\"><div class=\"math-block__status\">Rendering formula...</div><pre class=\"math-block__source hidden\">{}</pre><div class=\"math-block__formula\"></div></div>",
        escape_html(source)
    )
}

struct HighlightedCodeBlock {
    language_label: Option<String>,
    lines_html: Vec<String>,
    line_count: usize,
}

struct ResolvedLanguage<'a> {
    badge_label: Option<&'a str>,
    syntax: &'static SyntaxReference,
}

fn highlight_code(language: Option<&str>, source: &str) -> HighlightedCodeBlock {
    let assets = highlight_assets();
    let resolved_language = resolve_language(&assets.syntax_set, language);
    let mut highlighter = HighlightLines::new(resolved_language.syntax, assets.theme());
    let mut lines_html = Vec::new();

    for line in LinesWithEndings::from(source) {
        let line_html = highlighter
            .highlight_line(line, &assets.syntax_set)
            .ok()
            .and_then(|regions| {
                styled_line_to_highlighted_html(&regions, IncludeBackground::No).ok()
            })
            .unwrap_or_else(|| escape_html(trim_line_ending(line)));
        lines_html.push(render_code_line(&line_html));
    }

    if lines_html.is_empty() {
        lines_html.push(render_code_line(""));
    }

    HighlightedCodeBlock {
        language_label: resolved_language.badge_label.map(ToOwned::to_owned),
        line_count: lines_html.len(),
        lines_html,
    }
}

fn resolve_language<'a>(
    syntax_set: &'static SyntaxSet,
    language: Option<&'a str>,
) -> ResolvedLanguage<'a> {
    let badge_label = language;
    let syntax = language
        .and_then(|value| resolve_syntax(syntax_set, value))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    ResolvedLanguage {
        badge_label,
        syntax,
    }
}

pub(crate) fn resolve_syntax(
    syntax_set: &'static SyntaxSet,
    language: &str,
) -> Option<&'static SyntaxReference> {
    let candidates = syntax_candidates(language);

    candidates
        .into_iter()
        .find_map(|value| syntax_set.find_syntax_by_token(value))
        .or_else(|| {
            syntax_candidates(language)
                .into_iter()
                .find_map(|value| syntax_set.find_syntax_by_name(value))
        })
}

fn syntax_candidates(language: &str) -> Vec<&str> {
    match language {
        "python" => vec!["python", "py"],
        "javascript" => vec!["javascript", "js"],
        "typescript" => vec!["typescript", "ts", "javascript", "js"],
        "swift" => vec!["swift", "rust"],
        "kotlin" => vec!["kotlin", "kt", "java"],
        "cpp" => vec!["cpp", "c++"],
        "bash" => vec!["bash", "shell", "sh"],
        "yaml" => vec!["yaml", "yml"],
        "csharp" => vec!["csharp", "c#"],
        other => vec![other],
    }
}

fn render_code_line(line_html: &str) -> String {
    let content = if line_html.is_empty() {
        "&nbsp;"
    } else {
        line_html
    };
    format!("<span class=\"code-block__line\">{content}</span>")
}

fn render_line_numbers(line_count: usize) -> String {
    (1..=line_count)
        .map(|line_number| format!("<span class=\"code-block__line-number\">{line_number}</span>"))
        .collect()
}

fn normalize_code_block_source(source: &str) -> String {
    source.trim_end_matches(['\r', '\n']).to_string()
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn escape_html_attribute(value: &str) -> String {
    escape_html(value)
}

pub(crate) struct HighlightAssets {
    pub(crate) syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl HighlightAssets {
    fn theme(&self) -> &Theme {
        self.theme_set
            .themes
            .get("base16-ocean.dark")
            .or_else(|| self.theme_set.themes.get("InspiredGitHub"))
            .or_else(|| self.theme_set.themes.values().next())
            .expect("syntect should provide at least one default theme")
    }
}

pub(crate) fn highlight_assets() -> &'static HighlightAssets {
    static HIGHLIGHT_ASSETS: OnceLock<HighlightAssets> = OnceLock::new();

    HIGHLIGHT_ASSETS.get_or_init(|| HighlightAssets {
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme_set: ThemeSet::load_defaults(),
    })
}
