use super::implementation::{extract_title, highlight_assets, render_html, resolve_syntax};

#[test]
fn extracts_first_heading_as_title() {
    let markdown = "intro\n# Hello\n## Next";
    assert_eq!(extract_title(markdown).as_deref(), Some("Hello"));
}

#[test]
fn renders_tables() {
    let markdown = "| A | B |\n| - | - |\n| 1 | 2 |";
    let html = render_html(markdown);
    assert!(html.contains("<table>"));
    assert!(html.contains("<td>1</td>"));
}

#[test]
fn renders_highlighted_code_blocks_with_line_numbers() {
    let markdown = "```rust\nfn main() {\n    println!(\"hi\");\n}\n```";
    let html = render_html(markdown);

    assert!(html.contains("class=\"code-block\""));
    assert!(html.contains("data-language=\"rust\""));
    assert!(html.contains("class=\"code-block__badge\">rust</div>"));
    assert!(html.contains("class=\"code-block__line-number\">1</span>"));
    assert!(html.contains("class=\"code-block__line-number\">3</span>"));
    assert!(html.contains("style=\""));
}

#[test]
fn renders_mermaid_blocks_separately_from_code_highlighting() {
    let markdown = "```mermaid\nflowchart TD\nA --> B\n```";
    let html = render_html(markdown);

    assert!(html.contains("class=\"mermaid-block\""));
    assert!(html.contains("class=\"mermaid-block__diagram\""));
    assert!(html.contains("flowchart TD"));
    assert!(!html.contains("class=\"code-block\""));
}

#[test]
fn renders_inline_and_display_math() {
    let markdown = "Inline math $e^{i\\pi}+1=0$.\n\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$";
    let html = render_html(markdown);

    assert!(html.contains("class=\"math math-inline\""));
    assert!(html.contains("e^{i\\pi}+1=0"));
    assert!(html.contains("class=\"math math-display\""));
    assert!(html.contains("\\int_0^1 x^2 dx = \\frac{1}{3}"));
}

#[test]
fn renders_fenced_math_blocks_separately_from_code_highlighting() {
    let markdown = "```math\n\\left( \\sum_{k=1}^n a_k b_k \\right)^2\n```";
    let html = render_html(markdown);

    assert!(html.contains("class=\"math-block\""));
    assert!(html.contains("class=\"math-block__formula\""));
    assert!(html.contains("\\left( \\sum_{k=1}^n a_k b_k \\right)^2"));
    assert!(!html.contains("class=\"code-block\""));
}

#[test]
fn falls_back_safely_for_unknown_languages() {
    let markdown = "```unknownlang\n<tag>\n```";
    let html = render_html(markdown);

    assert!(html.contains("data-language=\"unknownlang\""));
    assert!(html.contains("&lt;tag&gt;"));
    assert!(!html.contains("<tag>"));
}

#[test]
fn resolves_typescript_swift_and_kotlin_syntaxes() {
    let syntax_set = &highlight_assets().syntax_set;

    assert_eq!(
        resolve_syntax(syntax_set, "typescript").map(|syntax| syntax.name.as_str()),
        Some("JavaScript")
    );
    assert_eq!(
        resolve_syntax(syntax_set, "swift").map(|syntax| syntax.name.as_str()),
        Some("Rust")
    );
    assert_eq!(
        resolve_syntax(syntax_set, "kotlin").map(|syntax| syntax.name.as_str()),
        Some("Java")
    );
}

#[test]
fn resolves_alias_tokens_for_cpp_bash_and_yaml() {
    let syntax_set = &highlight_assets().syntax_set;

    assert_eq!(
        resolve_syntax(syntax_set, "cpp").map(|syntax| syntax.name.as_str()),
        Some("C++")
    );
    assert_eq!(
        resolve_syntax(syntax_set, "bash").map(|syntax| syntax.name.as_str()),
        Some("Bourne Again Shell (bash)")
    );
    assert_eq!(
        resolve_syntax(syntax_set, "yaml").map(|syntax| syntax.name.as_str()),
        Some("YAML")
    );
}

#[test]
fn preserves_blank_lines_in_code_blocks() {
    let markdown = "```text\nalpha\n\nomega\n```";
    let html = render_html(markdown);

    assert_eq!(
        html.matches("class=\"code-block__line-number\">").count(),
        3
    );
    assert_eq!(html.matches("class=\"code-block__line\">").count(), 3);
}

#[test]
fn leaves_inline_code_unchanged() {
    let html = render_html("Use `cargo test`.");

    assert!(html.contains("<code>cargo test</code>"));
    assert!(!html.contains("code-block__badge"));
}

#[test]
fn example_languages_keeps_mainstream_highlight_blocks() {
    let html = render_html(include_str!("../../examples/languages.md"));

    assert!(html.contains("data-language=\"typescript\""));
    assert!(html.contains("data-language=\"swift\""));
    assert!(html.contains("data-language=\"kotlin\""));
    assert!(html.contains("class=\"code-block__line-number\">1</span>"));
}

#[test]
fn example_mermaid_keeps_mermaid_render_containers() {
    let html = render_html(include_str!("../../examples/mermaid.md"));

    assert!(html.contains("class=\"mermaid-block\""));
    assert!(html.contains("class=\"mermaid-block__diagram\""));
}

#[test]
fn example_math_keeps_math_render_containers() {
    let html = render_html(include_str!("../../examples/math.md"));

    assert!(html.contains("class=\"math math-inline\""));
    assert!(html.contains("class=\"math math-display\""));
    assert!(html.contains("class=\"math-block\""));
}
