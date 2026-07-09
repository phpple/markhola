use pulldown_cmark::{Options, Parser, html};

pub fn render_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
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

#[cfg(test)]
mod tests {
    use super::{extract_title, render_html};

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
}
