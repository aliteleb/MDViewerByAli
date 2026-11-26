use std::path::{Path, PathBuf};

use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use serde::Serialize;

use crate::folder::normalize_path_str;

#[derive(Debug, Clone, Serialize)]
pub struct TocItem {
    pub id: String,
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocStats {
    pub words: usize,
    pub read_minutes: usize,
    pub size_bytes: u64,
    pub size_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedDoc {
    pub path: String,
    pub filename: String,
    pub is_mermaid: bool,
    pub html: String,
    pub raw_markdown: String,
    pub toc: Vec<TocItem>,
    pub stats: DocStats,
}

pub fn is_supported_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "mmd" | "mermaid" | "txt"
    )
}

pub fn is_mermaid_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "mmd" | "mermaid")
}

pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn heading_level_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn url_encode_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn resolve_local_image_url(dest: &str, base_dir: Option<&Path>) -> Option<String> {
    let trimmed = dest.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
        || trimmed.starts_with('#')
    {
        return None;
    }

    let (before_frag, fragment) = match trimmed.split_once('#') {
        Some((before, frag)) => (before, Some(frag)),
        None => (trimmed, None),
    };
    let raw_path = before_frag.split('?').next().unwrap_or(before_frag);
    let candidate = Path::new(raw_path);
    let resolved: PathBuf = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else if let Some(dir) = base_dir {
        dir.join(candidate)
    } else {
        return None;
    };

    let encoded = url_encode_path(&resolved.to_string_lossy());
    let mut url = format!("http://mdviewer.localhost/local-file?path={encoded}");
    if let Some(frag) = fragment
        && !frag.is_empty()
    {
        url.push('#');
        url.push_str(frag);
    }
    Some(url)
}

fn slugify_heading(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in text.trim().chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn trim_matched_url(raw: &str) -> &str {
    let mut end = raw.len();
    while end > 0 {
        let candidate = &raw[..end];
        let Some(ch) = candidate.chars().next_back() else {
            break;
        };
        let strip = if matches!(
            ch,
            '.' | ',' | ';' | ':' | '!' | '?' | '*' | '~' | '_' | '\'' | '"'
        ) {
            true
        } else if ch == ')' {
            candidate.matches(')').count() > candidate.matches('(').count()
        } else if ch == ']' {
            candidate.matches(']').count() > candidate.matches('[').count()
        } else if ch == '}' {
            candidate.matches('}').count() > candidate.matches('{').count()
        } else {
            false
        };
        if strip {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    &raw[..end]
}

fn emit_plain_with_emoji(plain: &str, out: &mut Vec<Event<'_>>) {
    let replaced = crate::emoji::replace_emoji_shortcodes(plain);
    out.push(Event::Text(CowStr::from(replaced.into_owned())));
}

fn try_match_bare_email(text: &str, pos: usize) -> Option<&str> {
    let rest = &text[pos..];
    let bytes = rest.as_bytes();
    let first = *bytes.first()?;
    if !(first.is_ascii_alphanumeric() || matches!(first, b'_' | b'-')) {
        return None;
    }

    let valid_boundary = text[..pos]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric() && !matches!(c, '_' | '@' | '.' | '/'));
    if !valid_boundary {
        return None;
    }

    let mut i = 0usize;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_' | b'%' | b'+' | b'-'))
    {
        i += 1;
    }
    if i == 0 || i >= bytes.len() || bytes[i] != b'@' {
        return None;
    }
    let local = &rest[..i];
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return None;
    }

    let dom_start = i + 1;
    let mut j = dom_start;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'.' | b'-')) {
        j += 1;
    }
    while j > dom_start && matches!(bytes[j - 1], b'.' | b'-') {
        j -= 1;
    }
    let domain = &rest[dom_start..j];
    if !domain.contains('.') || domain.starts_with('.') || domain.contains("..") {
        return None;
    }
    let tld = domain.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if let Some(next_ch) = rest[j..].chars().next()
        && (next_ch.is_alphanumeric() || matches!(next_ch, '@' | '/'))
    {
        return None;
    }
    Some(&rest[..j])
}

fn push_autolinked_text(text: &str, out: &mut Vec<Event<'_>>) {
    let mut last_emitted = 0usize;
    let mut pos = 0usize;

    while pos < text.len() {
        let rest = &text[pos..];
        let bytes = rest.as_bytes();
        let matched_scheme = if bytes.get(..8).is_some_and(|b| b.eq_ignore_ascii_case(b"https://")) {
            Some(("https://", false))
        } else if bytes.get(..7).is_some_and(|b| b.eq_ignore_ascii_case(b"http://")) {
            Some(("http://", false))
        } else if bytes.get(..4).is_some_and(|b| b.eq_ignore_ascii_case(b"www.")) {
            Some(("www.", true))
        } else {
            None
        };

        let valid_boundary = text[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');

        if let Some((prefix, prepend_https)) = matched_scheme
            && valid_boundary
        {
            let mut raw_end = text.len();
            for (off, ch) in rest.char_indices() {
                if ch.is_whitespace()
                    || matches!(ch, '<' | '>' | '"' | '\'' | '`' | '«' | '»' | '،' | '؛')
                {
                    raw_end = pos + off;
                    break;
                }
            }

            let candidate = trim_matched_url(&text[pos..raw_end]);
            let after_prefix = &candidate[prefix.len()..];
            let host = after_prefix.split(['/', '?', '#']).next().unwrap_or("");
            let valid_host = if prepend_https {
                !host.is_empty() && host.contains('.')
            } else {
                !host.is_empty()
                    && (host.contains('.')
                        || host.eq_ignore_ascii_case("localhost")
                        || host.to_ascii_lowercase().starts_with("localhost:"))
            };

            if valid_host {
                if pos > last_emitted {
                    emit_plain_with_emoji(&text[last_emitted..pos], out);
                }
                let href = if prepend_https {
                    format!("https://{candidate}")
                } else {
                    candidate.to_string()
                };
                out.push(Event::Html(CowStr::from(format!(
                    "<a href=\"{}\">{}</a>",
                    html_escape(&href),
                    html_escape(candidate)
                ))));
                let consumed_end = pos + candidate.len();
                last_emitted = consumed_end;
                pos = consumed_end;
                continue;
            }
        } else if let Some(email) = try_match_bare_email(text, pos) {
            if pos > last_emitted {
                emit_plain_with_emoji(&text[last_emitted..pos], out);
            }
            let escaped = html_escape(email);
            out.push(Event::Html(CowStr::from(format!(
                "<a href=\"mailto:{escaped}\">{escaped}</a>"
            ))));
            let consumed_end = pos + email.len();
            last_emitted = consumed_end;
            pos = consumed_end;
            continue;
        }

        let step = rest.chars().next().map_or(1, char::len_utf8);
        pos += step;
    }

    if last_emitted == 0 {
        emit_plain_with_emoji(text, out);
    } else if last_emitted < text.len() {
        emit_plain_with_emoji(&text[last_emitted..], out);
    }
}

fn render_frontmatter_html(raw: &str) -> String {
    let mut entries: Vec<(String, String)> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if (line.starts_with(' ') || line.starts_with('\t')) && trimmed.starts_with('-') {
            let item = trimmed.trim_start_matches('-').trim();
            if !item.is_empty()
                && let Some((_, prev_val)) = entries.last_mut()
            {
                if prev_val.is_empty() {
                    prev_val.push_str(item);
                } else {
                    prev_val.push_str(", ");
                    prev_val.push_str(item);
                }
            }
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim();
            if !key.is_empty() {
                let mut val = v.trim();
                if ((val.starts_with('"') && val.ends_with('"'))
                    || (val.starts_with('\'') && val.ends_with('\'')))
                    && val.len() >= 2
                {
                    val = &val[1..val.len() - 1];
                }
                entries.push((key.to_string(), val.to_string()));
            }
        }
    }

    if entries.is_empty() {
        let escaped = html_escape(raw.trim());
        return format!("<div class=\"mdv-frontmatter\"><pre><code>{escaped}</code></pre></div>\n");
    }

    let mut rows = String::new();
    for (k, v) in entries {
        rows.push_str("<tr><th>");
        rows.push_str(&html_escape(&k));
        rows.push_str("</th><td>");
        rows.push_str(&html_escape(&v));
        rows.push_str("</td></tr>");
    }
    format!(
        "<div class=\"mdv-frontmatter\"><div class=\"mdv-frontmatter-label\">Metadata</div><table><thead><tr><th>Property</th><th>Value</th></tr></thead><tbody>{rows}</tbody></table></div>\n"
    )
}

fn render_diff_block(code: &str) -> String {
    let trimmed = code.strip_suffix('\n').unwrap_or(code);
    let mut lines_html = String::with_capacity(code.len() * 2);
    for line in trimmed.split('\n') {
        let clean = line.strip_suffix('\r').unwrap_or(line);
        let cls = if clean.starts_with('+') && !clean.starts_with("+++") {
            "diff-line diff-add"
        } else if clean.starts_with('-') && !clean.starts_with("---") {
            "diff-line diff-del"
        } else if clean.starts_with("@@") {
            "diff-line diff-hunk"
        } else {
            "diff-line"
        };
        lines_html.push_str("<span class=\"");
        lines_html.push_str(cls);
        lines_html.push_str("\">");
        lines_html.push_str(&html_escape(clean));
        lines_html.push_str("\n</span>");
    }
    format!(
        "<div class=\"code-block\"><div class=\"code-header\"><span class=\"code-lang\">diff</span><button type=\"button\" class=\"copy-btn\" title=\"Copy code\">Copy</button></div><pre><code class=\"language-diff mdv-diff\">{lines_html}</code></pre></div>\n"
    )
}

fn alert_meta(kind: BlockQuoteKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BlockQuoteKind::Note => (
            "note",
            "Note",
            r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"#,
        ),
        BlockQuoteKind::Tip => (
            "tip",
            "Tip",
            r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z"/></svg>"#,
        ),
        BlockQuoteKind::Important => (
            "important",
            "Important",
            r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"#,
        ),
        BlockQuoteKind::Warning => (
            "warning",
            "Warning",
            r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"#,
        ),
        BlockQuoteKind::Caution => (
            "caution",
            "Caution",
            r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"#,
        ),
    }
}

pub fn render_document(content: &str, path: &Path) -> RenderedDoc {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled.md")
        .to_string();
    let path_str = normalize_path_str(path);
    let size_bytes = content.len() as u64;
    let words = content.split_whitespace().count();
    let read_minutes = (words / 200).max(1);
    let stats = DocStats {
        words,
        read_minutes,
        size_bytes,
        size_label: format_size(size_bytes),
    };

    if is_mermaid_file(path) {
        let html = format!("<div class=\"mermaid\">{}</div>", html_escape(content.trim()));
        return RenderedDoc {
            path: path_str,
            filename,
            is_mermaid: true,
            html,
            raw_markdown: content.to_string(),
            toc: Vec::new(),
            stats,
        };
    }

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;

    let base_dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let parser = Parser::new_ext(content, options);

    let mut toc = Vec::new();
    let mut transformed: Vec<Event<'_>> = Vec::new();

    let mut in_metadata = false;
    let mut metadata_buffer = String::new();

    let mut in_heading: Option<u8> = None;
    let mut heading_events: Vec<Event<'_>> = Vec::new();
    let mut heading_text = String::new();

    let mut in_code_block: Option<String> = None;
    let mut code_buffer = String::new();
    let mut quote_stack: Vec<bool> = Vec::new();
    let mut in_link: usize = 0;

    for event in parser {
        if in_metadata {
            match event {
                Event::End(TagEnd::MetadataBlock(_)) => {
                    in_metadata = false;
                    transformed.push(Event::Html(CowStr::from(render_frontmatter_html(
                        &metadata_buffer,
                    ))));
                    metadata_buffer.clear();
                }
                Event::Text(t) => metadata_buffer.push_str(&t),
                Event::SoftBreak | Event::HardBreak => metadata_buffer.push('\n'),
                _ => {}
            }
            continue;
        }

        if let Some(level) = in_heading {
            match event {
                Event::End(TagEnd::Heading(_)) => {
                    let id = format!("h-{}", toc.len());
                    let text = heading_text.trim().to_string();
                    let slug = slugify_heading(&text);
                    let escaped_slug = html_escape(&slug);
                    toc.push(TocItem {
                        id: id.clone(),
                        level,
                        text,
                    });

                    let mut inner_html = String::new();
                    pulldown_cmark::html::push_html(&mut inner_html, heading_events.drain(..));
                    transformed.push(Event::Html(CowStr::from(format!(
                        "<h{level} id=\"{id}\" data-slug=\"{escaped_slug}\" dir=\"auto\"><a class=\"mdv-heading-anchor\" href=\"#{escaped_slug}\" title=\"Copy section link\" aria-hidden=\"true\">#</a>{inner_html}</h{level}>\n"
                    ))));
                    heading_text.clear();
                    in_heading = None;
                }
                ev => {
                    match ev {
                        Event::Text(t) => {
                            let replaced = crate::emoji::replace_emoji_shortcodes(&t);
                            heading_text.push_str(&replaced);
                            heading_events.push(Event::Text(CowStr::from(replaced.into_owned())));
                        }
                        Event::InlineMath(tex) => {
                            heading_text.push_str(&tex);
                            let escaped = html_escape(&tex);
                            heading_events.push(Event::Html(CowStr::from(format!(
                                "<span class=\"mdv-math-inline\">{escaped}</span>"
                            ))));
                        }
                        Event::Code(t) => {
                            heading_text.push_str(&t);
                            heading_events.push(Event::Code(t));
                        }
                        Event::SoftBreak | Event::HardBreak => {
                            heading_text.push(' ');
                            heading_events.push(ev);
                        }
                        other => heading_events.push(other),
                    }
                }
            }
            continue;
        }

        if let Some(lang) = in_code_block.as_ref() {
            match event {
                Event::End(TagEnd::CodeBlock) => {
                    let lang_clean = lang.trim();
                    let lang_token = lang_clean.split_whitespace().next().unwrap_or("");
                    if lang_token.eq_ignore_ascii_case("mermaid") {
                        let escaped = html_escape(code_buffer.trim());
                        transformed.push(Event::Html(CowStr::from(format!(
                            "<div class=\"mermaid\">{escaped}</div>\n"
                        ))));
                    } else if lang_token.eq_ignore_ascii_case("math")
                        || lang_token.eq_ignore_ascii_case("latex")
                    {
                        let escaped = html_escape(code_buffer.trim());
                        transformed.push(Event::Html(CowStr::from(format!(
                            "<div class=\"mdv-math-display\">{escaped}</div>\n"
                        ))));
                    } else if lang_token.eq_ignore_ascii_case("diff") {
                        transformed.push(Event::Html(CowStr::from(render_diff_block(
                            &code_buffer,
                        ))));
                    } else {
                        let lang_label = if lang_token.is_empty() {
                            "text"
                        } else {
                            lang_token
                        };
                        let class_attr = if lang_token.is_empty() {
                            String::new()
                        } else {
                            format!(" class=\"language-{}\"", html_escape(lang_token))
                        };
                        let escaped_code = html_escape(&code_buffer);
                        let escaped_label = html_escape(lang_label);
                        transformed.push(Event::Html(CowStr::from(format!(
                            "<div class=\"code-block\"><div class=\"code-header\"><span class=\"code-lang\">{escaped_label}</span><button type=\"button\" class=\"copy-btn\" title=\"Copy code\">Copy</button></div><pre><code{class_attr}>{escaped_code}</code></pre></div>\n"
                        ))));
                    }
                    code_buffer.clear();
                    in_code_block = None;
                }
                Event::Text(t) => code_buffer.push_str(&t),
                Event::SoftBreak | Event::HardBreak => code_buffer.push('\n'),
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::MetadataBlock(_)) => {
                in_metadata = true;
                metadata_buffer.clear();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(heading_level_num(level));
                heading_events.clear();
                heading_text.clear();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code_buffer.clear();
                in_code_block = Some(lang);
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let final_url = match resolve_local_image_url(&dest_url, base_dir) {
                    Some(local) => CowStr::from(local),
                    None => dest_url,
                };
                transformed.push(Event::Start(Tag::Image {
                    link_type,
                    dest_url: final_url,
                    title,
                    id,
                }));
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                if let Some(alert_kind) = kind {
                    let (cls, title, icon_svg) = alert_meta(alert_kind);
                    quote_stack.push(true);
                    transformed.push(Event::Html(CowStr::from(format!(
                        "<div class=\"mdv-alert mdv-alert-{cls}\" role=\"note\"><div class=\"mdv-alert-title\">{icon_svg}<span>{title}</span></div>\n"
                    ))));
                } else {
                    quote_stack.push(false);
                    transformed.push(Event::Start(Tag::BlockQuote(None)));
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if quote_stack.pop().unwrap_or(false) {
                    transformed.push(Event::Html(CowStr::from("</div>\n")));
                } else {
                    transformed.push(Event::End(TagEnd::BlockQuote(None)));
                }
            }
            Event::InlineMath(tex) => {
                let escaped = html_escape(&tex);
                transformed.push(Event::Html(CowStr::from(format!(
                    "<span class=\"mdv-math-inline\">{escaped}</span>"
                ))));
            }
            Event::DisplayMath(tex) => {
                let escaped = html_escape(tex.trim());
                transformed.push(Event::Html(CowStr::from(format!(
                    "<div class=\"mdv-math-display\">{escaped}</div>\n"
                ))));
            }
            Event::Start(Tag::Link { .. }) => {
                in_link += 1;
                transformed.push(event);
            }
            Event::End(TagEnd::Link) => {
                in_link = in_link.saturating_sub(1);
                transformed.push(event);
            }
            Event::Text(text) if in_link == 0 => {
                push_autolinked_text(&text, &mut transformed);
            }
            Event::Text(text) => {
                emit_plain_with_emoji(&text, &mut transformed);
            }
            other => transformed.push(other),
        }
    }

    let mut html = String::with_capacity(content.len() * 2);
    pulldown_cmark::html::push_html(&mut html, transformed.into_iter());

    RenderedDoc {
        path: path_str,
        filename,
        is_mermaid: false,
        html,
        raw_markdown: content.to_string(),
        toc,
        stats,
    }
}

pub fn demo_markdown() -> &'static str {
    r#"# MDViewerByAli — Offline Markdown & Mermaid Viewer

مرحباً بك في **MDViewerByAli**! هذا مستند تجريبي يوضح دعم اللغة العربية والإنجليزية معاً، الفهرس الجانبي التفاعلي، الجداول، تلوين الأكواد، ومخططات **Mermaid** مع التكبير والتصغير.

## نظرة سريعة على المزايا (Features)

- [x] قراءة ملفات `.md`, `.markdown`, `.mmd`, `.mermaid` بسرعة عالية وبدون حد للحجم
- [x] **Table of Contents** تفاعلي + **File Tree** لتصفح مجلدات المشاريع (`Ctrl+Shift+O`)
- [x] دعم **Mermaid** و **KaTeX Math** (`$inline$` / `$$display$$`) بدون إنترنت
- [x] تنبيهات **GitHub Alerts** (`[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`)
- [x] تكبير المخططات والصور (**Lightbox** Zoom / Pan) وتصدير **PDF** عبر `Ctrl + P`

> [!TIP]
> جرب سحب أي ملف أو مجلد وإفلاته داخل النافذة، أو اضغط على أي معادلة أو مخطط لتجربة العرض الاحترافي!

## Math Formulas (KaTeX)

تدعم المعادلات داخل السطر مثل $E = mc^2$ والمعادلات المنفصلة أوفلاين بالكامل:

$$
\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi} \quad\text{and}\quad x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
$$

## Architecture Diagram (Mermaid)

اضغط على المخطط بالأسفل لفتحه بملء الشاشة وتكبيره بعجلة الماوس:

```mermaid
flowchart LR
    File[".md / .mermaid File"] --> RustCore["Rust Core (pulldown-cmark + notify)"]
    RustCore --> ToC["Table of Contents & Stats"]
    RustCore --> WebView["WebView2 Renderer (Offline)"]
    WebView --> Mermaid["Mermaid SVG + Lightbox"]
    WebView --> Highlight["Code Highlighting & Copy"]
```

## Comparison Table

| Feature | Online Viewer | MDViewerByAli |
| :--- | :---: | :---: |
| File Size Limit | 512 KB | **Unlimited** |
| Works 100% Offline | No | **Yes** |
| Multi-Tab & Live Reload | No | **Yes** |
| Arabic / RTL Support | Basic | **Native BiDi** |
| Mermaid Zoom Lightbox | Yes | **Yes** |

## Code Sample (Rust)

```rust
fn render_markdown(source: &str) -> String {
    let parser = pulldown_cmark::Parser::new(source);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}
```

> **Tip:** جرب سحب أي ملف `.md` أو `.mermaid` وإفلاته داخل النافذة مباشرة، أو اضغط `Ctrl+F` للبحث داخل المستند.
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_tables_and_tasklists() {
        let md = "- [x] Done\n- [ ] Todo\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = render_document(md, Path::new("test.md"));
        assert!(doc.html.contains("<table>"));
        assert!(doc.html.contains("type=\"checkbox\""));
        assert!(doc.html.contains("checked"));
    }

    #[test]
    fn extracts_bilingual_toc_headings() {
        let md = "# عنوان رئيسي\n## Sub Section\n### تفاصيل\n";
        let doc = render_document(md, Path::new("bilingual.md"));
        assert_eq!(doc.toc.len(), 3);
        assert_eq!(doc.toc[0].id, "h-0");
        assert_eq!(doc.toc[0].level, 1);
        assert_eq!(doc.toc[0].text, "عنوان رئيسي");
        assert_eq!(doc.toc[1].id, "h-1");
        assert_eq!(doc.toc[1].level, 2);
        assert_eq!(doc.toc[1].text, "Sub Section");
        assert!(
            doc.html
                .contains(r##"<h1 id="h-0" data-slug="عنوان-رئيسي" dir="auto"><a class="mdv-heading-anchor" href="#عنوان-رئيسي" title="Copy section link" aria-hidden="true">#</a>عنوان رئيسي</h1>"##)
        );
    }

    #[test]
    fn escapes_mermaid_blocks_preserving_arrows() {
        let md = "```mermaid\ngraph LR\n  A --> B\n```\n";
        let doc = render_document(md, Path::new("diag.md"));
        assert!(doc.html.contains("<div class=\"mermaid\">graph LR\n  A --&gt; B</div>"));
    }

    #[test]
    fn renders_standalone_mmd_file_without_toc() {
        let mmd = "sequenceDiagram\n  Alice->>Bob: Hi\n";
        let doc = render_document(mmd, Path::new("flow.mmd"));
        assert!(doc.is_mermaid);
        assert!(doc.toc.is_empty());
        assert!(doc.html.contains("<div class=\"mermaid\">sequenceDiagram"));
    }

    #[test]
    fn rewrites_relative_image_paths() {
        let md = "![diagram](images/arch.png)\n";
        let doc = render_document(md, Path::new(r"E:\Docs\readme.md"));
        assert!(doc.html.contains("http://mdviewer.localhost/local-file?path="));
        assert!(doc.html.contains("arch.png"));
    }

    #[test]
    fn renders_inline_and_display_math() {
        let md = "Formula $E = mc^2$ here.\n\n$$\na^2 + b^2 = c^2\n$$\n";
        let doc = render_document(md, Path::new("math.md"));
        assert!(
            doc.html
                .contains(r#"<span class="mdv-math-inline">E = mc^2</span>"#)
        );
        assert!(
            doc.html
                .contains(r#"<div class="mdv-math-display">a^2 + b^2 = c^2</div>"#)
        );
    }

    #[test]
    fn renders_gfm_alerts_and_normal_blockquotes() {
        let md = "> [!TIP]\n> Useful advice\n\n> Plain quote\n\n> [!CAUTION]\n> > Nested quote\n";
        let doc = render_document(md, Path::new("alerts.md"));
        assert!(doc.html.contains(r#"<div class="mdv-alert mdv-alert-tip""#));
        assert!(doc.html.contains("<span>Tip</span>"));
        assert!(doc.html.contains("<blockquote>\n<p>Plain quote</p>\n</blockquote>"));
        assert!(doc.html.contains(r#"<div class="mdv-alert mdv-alert-caution""#));
        assert!(doc.html.contains("<blockquote>\n<p>Nested quote</p>\n</blockquote>"));
    }

    #[test]
    fn autolinks_bare_urls_in_paragraphs_and_tables() {
        let md = "| Screen | URL |\n|---|---|\n| Sales | https://test1.erptestserver387621358.cloud/inventory/inventory-sales-register |\n\nVisit www.example.com. Or [already linked](https://example.org).\n";
        let doc = render_document(md, Path::new("links.md"));
        assert!(doc.html.contains(
            r#"<a href="https://test1.erptestserver387621358.cloud/inventory/inventory-sales-register">https://test1.erptestserver387621358.cloud/inventory/inventory-sales-register</a>"#
        ));
        assert!(doc.html.contains(
            r#"<a href="https://www.example.com">www.example.com</a>. Or <a href="https://example.org">already linked</a>."#
        ));
    }

    #[test]
    fn adds_heading_data_slug_for_anchors() {
        let md = "# First Heading!\n\n## القسم الأول\n";
        let doc = render_document(md, Path::new("slugs.md"));
        assert!(doc.html.contains(r#"id="h-0" data-slug="first-heading""#));
        assert!(doc.html.contains(r#"id="h-1" data-slug="القسم-الأول""#));
    }

    #[test]
    fn renders_yaml_frontmatter_as_table() {
        let md = "---\ntitle: \"Guide\"\ntags:\n  - rust\n  - markdown\n---\n\n# Body\n";
        let doc = render_document(md, Path::new("fm.md"));
        assert!(doc.html.contains(r#"<div class="mdv-frontmatter">"#));
        assert!(doc.html.contains("<th>title</th><td>Guide</td>"));
        assert!(doc.html.contains("<th>tags</th><td>rust, markdown</td>"));
    }

    #[test]
    fn renders_fenced_math_and_diff_blocks() {
        let md = "```math\ny = mx + b\n```\n\n```diff\n-old line\n+new line\n@@ -1,2 +1,2 @@\n```\n";
        let doc = render_document(md, Path::new("blocks.md"));
        assert!(doc.html.contains(r#"<div class="mdv-math-display">y = mx + b</div>"#));
        assert!(doc.html.contains(r#"class="language-diff mdv-diff""#));
        assert!(doc.html.contains(r#"<span class="diff-line diff-del">-old line"#));
        assert!(doc.html.contains(r#"<span class="diff-line diff-add">+new line"#));
        assert!(doc.html.contains(r#"<span class="diff-line diff-hunk">@@ -1,2 +1,2 @@"#));
    }

    #[test]
    fn replaces_emoji_shortcodes_and_autolinks_emails() {
        let md = ":rocket: Contact ali@example.com or visit https://example.com:8080/app\n";
        let doc = render_document(md, Path::new("emoji_mail.md"));
        assert!(doc.html.contains("🚀"));
        assert!(doc.html.contains(r#"<a href="mailto:ali@example.com">ali@example.com</a>"#));
        assert!(doc.html.contains(r#"<a href="https://example.com:8080/app">https://example.com:8080/app</a>"#));
    }

    #[test]
    fn preserves_theme_fragment_on_local_images() {
        let md = "![logo](logo-dark.png#gh-dark-mode-only)\n";
        let doc = render_document(md, Path::new(r"E:\Docs\readme.md"));
        assert!(doc.html.contains("local-file?path="));
        assert!(doc.html.contains("#gh-dark-mode-only"));
    }
}
