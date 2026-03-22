use pulldown_cmark::{Parser, Options, Event, Tag, TagEnd};

/// Common trait for message renderers.
pub trait Renderer {
    fn render(&self, text: &str) -> String;
}

/// Renderer for Telegram HTML format.
pub struct TelegramRenderer;

impl TelegramRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for TelegramRenderer {
    fn render(&self, text: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        // We don't enable tables or task lists as Telegram doesn't support them well via HTML
        
        let parser = Parser::new_ext(text, options);
        let mut html_output = String::with_capacity(text.len() * 3 / 2);

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Strong => html_output.push_str("<b>"),
                    Tag::Emphasis => html_output.push_str("<i>"),
                    Tag::Strikethrough => html_output.push_str("<s>"),
                    Tag::Link { dest_url, .. } => {
                        html_output.push_str(&format!("<a href=\"{}\">", self.escape(&dest_url)));
                    }
                    Tag::CodeBlock(_) => html_output.push_str("<pre>"),
                    Tag::Paragraph => (), // Telegram messages wrap automatically, but we might want \n\n
                    Tag::Heading { .. } => html_output.push_str("<b>"), // Headers as bold
                    Tag::List(_) => (),
                    Tag::Table(_) => (),
                    Tag::TableRow => (),
                    Tag::TableCell => (),
                    _ => (),
                },
                Event::End(tag) => match tag {
                    TagEnd::Strong => html_output.push_str("</b>"),
                    TagEnd::Emphasis => html_output.push_str("</i>"),
                    TagEnd::Strikethrough => html_output.push_str("</s>"),
                    TagEnd::Link => html_output.push_str("</a>"),
                    TagEnd::CodeBlock => html_output.push_str("</pre>"),
                    TagEnd::Paragraph => html_output.push_str("\n\n"),
                    TagEnd::Heading { .. } => html_output.push_str("</b>\n\n"),
                    TagEnd::Item => html_output.push_str("\n"),
                    TagEnd::List(_) => html_output.push_str("\n"),
                    TagEnd::Table => html_output.push_str("\n"),
                    TagEnd::TableRow => html_output.push_str("\n"),
                    TagEnd::TableCell => html_output.push_str(" | "),
                    _ => (),
                },
                Event::Text(text) => {
                    html_output.push_str(&self.escape(&text));
                }
                Event::Code(code) => {
                    html_output.push_str("<code>");
                    html_output.push_str(&self.escape(&code));
                    html_output.push_str("</code>");
                }
                Event::SoftBreak => html_output.push('\n'),
                Event::HardBreak => html_output.push_str("\n"),
                _ => (),
            }
        }

        html_output.trim().to_string()
    }
}

impl TelegramRenderer {

    fn escape(&self, text: &str) -> String {
        text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
    }
}

/// Renderer for Telegram MarkdownV2 format.
pub struct TelegramMarkdownV2Renderer;

impl TelegramMarkdownV2Renderer {
    pub fn new() -> Self {
        Self
    }

    /// Escape characters for MarkdownV2 normal text.
    /// Any character with code between 1 and 126 inclusively can be escaped anywhere with a preceding '\' character, 
    /// in which case it is treated as an ordinary character and not a part of the markup.
    fn escape_normal(&self, text: &str) -> String {
        let to_escape = ['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];
        let mut escaped = String::with_capacity(text.len() * 2);
        for c in text.chars() {
            if to_escape.contains(&c) {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        escaped
    }

    /// Escape characters for MarkdownV2 code blocks.
    /// Inside pre and code entities, all '`' and '\' characters must be escaped with a preceding '\' character.
    fn escape_code(&self, text: &str) -> String {
        text.replace("\\", "\\\\").replace("`", "\\`")
    }

    /// Escape characters for MarkdownV2 link URLs.
    /// Inside (...) part of inline link definition, all ')' and '\' must be escaped with a preceding '\' character.
    fn escape_link_url(&self, text: &str) -> String {
        text.replace("\\", "\\\\").replace(")", "\\)")
    }
}

impl Renderer for TelegramMarkdownV2Renderer {
    fn render(&self, text: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        
        let parser = Parser::new_ext(text, options);
        let mut md_output = String::with_capacity(text.len() * 3 / 2);
        let mut link_url = None;

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Strong => md_output.push('*'),
                    Tag::Emphasis => md_output.push('_'),
                    Tag::Strikethrough => md_output.push('~'),
                    Tag::Link { dest_url, .. } => {
                        link_url = Some(dest_url.to_string());
                        md_output.push('[');
                    }
                    Tag::CodeBlock(_) => md_output.push_str("```\n"),
                    Tag::Paragraph => (),
                    Tag::Heading { .. } => {
                        md_output.push('*');
                    }
                    Tag::Item => md_output.push_str("• "),
                    Tag::List(_) => (),
                    Tag::Table(_) => (),
                    Tag::TableRow => (),
                    Tag::TableCell => (),
                    _ => (),
                },
                Event::End(tag) => match tag {
                    TagEnd::Strong => md_output.push('*'),
                    TagEnd::Emphasis => md_output.push('_'),
                    TagEnd::Strikethrough => md_output.push('~'),
                    TagEnd::Link => {
                        md_output.push_str("](");
                        if let Some(url) = link_url.take() {
                            md_output.push_str(&self.escape_link_url(&url));
                        }
                        md_output.push(')');
                    }
                    TagEnd::CodeBlock => md_output.push_str("```\n"),
                    TagEnd::Paragraph => md_output.push_str("\n\n"),
                    TagEnd::Heading { .. } => md_output.push_str("*\n\n"),
                    TagEnd::Item => md_output.push('\n'),
                    TagEnd::List(_) => md_output.push('\n'),
                    TagEnd::Table => md_output.push('\n'),
                    TagEnd::TableRow => md_output.push('\n'),
                    TagEnd::TableCell => md_output.push_str(" | "),
                    _ => (),
                },
                Event::Text(text) => {
                    md_output.push_str(&self.escape_normal(&text));
                }
                Event::Code(code) => {
                    md_output.push('`');
                    md_output.push_str(&self.escape_code(&code));
                    md_output.push('`');
                }
                Event::SoftBreak => md_output.push('\n'),
                Event::HardBreak => md_output.push('\n'),
                _ => (),
            }
        }

        md_output.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_v2_escaping() {
        let renderer = TelegramMarkdownV2Renderer::new();
        
        // Normal text
        assert_eq!(renderer.render("Hello World!"), "Hello World\\!");
        assert_eq!(renderer.render("1.5.0"), "1\\.5\\.0");
        
        // Bold and Italic
        assert_eq!(renderer.render("**Bold**"), "*Bold*"); // pulldown-cmark Normalizes to single * for strong in some cases? 
        // Actually pulldown-cmark uses Tag::Strong for both ** and __. Telegram uses * for strong and _ for emphasis.
        
        // Code
        assert_eq!(renderer.render("`code`"), "`code`");
        assert_eq!(renderer.render("`code with \\ and `` `"), "`code with \\\\ and \\`\\` `");
        
        // Links
        assert_eq!(renderer.render("[Link](https://example.com)"), "[Link](https://example.com)");
    }
}
