//! Descriptions as structured content (spec: "Untrusted content").
//!
//! A description reaches the frontend as paragraphs of text pieces and link
//! pieces, never as markup. HTML is converted here: `<br>` and `<p>` become
//! line breaks, `<a href=u>x</a>` becomes a link piece, and every other tag is
//! dropped with its text kept. Only `http(s)://` and `mailto:` URLs become
//! links.

use serde::Serialize;

/// One piece of a paragraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Piece {
    Text {
        text: String,
    },
    /// A link to an `http(s)://` or `mailto:` URL. Its text can differ from
    /// the URL, in which case the frontend asks before opening it.
    Link {
        text: String,
        url: String,
    },
}

/// An Event's description, ready to show and to edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Description {
    /// One paragraph per line, for showing the description.
    pub paragraphs: Vec<Vec<Piece>>,
    /// The plain-text form, which the editor edits. A link whose text differs
    /// from its URL is written as `text (url)`, so that the URL survives an
    /// edit.
    pub text: String,
}

impl Description {
    pub(crate) fn from_plain_text(stored: &str) -> Self {
        let text = stored.replace("\r\n", "\n").replace('\r', "\n");
        Self::from_raw(vec![Raw::Text(text)])
    }

    pub(crate) fn from_html(stored: &str) -> Self {
        Self::from_raw(html_to_raw(stored))
    }

    fn from_raw(raw: Vec<Raw>) -> Self {
        let mut paragraphs = vec![Vec::new()];
        for piece in raw {
            match piece {
                Raw::Text(text) => {
                    for (index, line) in text.split('\n').enumerate() {
                        if index > 0 {
                            paragraphs.push(Vec::new());
                        }
                        let paragraph = paragraphs.last_mut().expect("never empty");
                        paragraph.extend(autolink(line));
                    }
                }
                Raw::Link { text, url } => paragraphs
                    .last_mut()
                    .expect("never empty")
                    .push(Piece::Link { text, url }),
            }
        }
        let mut paragraphs: Vec<Vec<Piece>> = paragraphs.into_iter().map(merge_text).collect();
        if paragraphs.iter().all(Vec::is_empty) {
            paragraphs.clear();
        }
        let text = paragraphs
            .iter()
            .map(|paragraph| paragraph.iter().map(plain_text).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        Self { paragraphs, text }
    }
}

/// Whether `url` may become a link and be opened in the browser: only
/// `http://`, `https://` and `mailto:` URLs with something after the scheme,
/// and nothing that could be read as two words or a second line.
pub fn link_allowed(url: &str) -> bool {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme) && url.len() > scheme.len())
}

/// A piece before it is split into lines and autolinked.
enum Raw {
    Text(String),
    Link { text: String, url: String },
}

fn plain_text(piece: &Piece) -> String {
    match piece {
        Piece::Text { text } => text.clone(),
        Piece::Link { text, url } if text == url => url.clone(),
        Piece::Link { text, url } => format!("{text} ({url})"),
    }
}

/// Joins neighbouring text pieces and drops empty ones.
fn merge_text(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut merged: Vec<Piece> = Vec::new();
    for piece in pieces {
        match (merged.last_mut(), piece) {
            (_, Piece::Text { text }) if text.is_empty() => {}
            (Some(Piece::Text { text: before }), Piece::Text { text }) => before.push_str(&text),
            (_, piece) => merged.push(piece),
        }
    }
    merged
}

/// Splits one line of text into text pieces and links to the URLs in it.
fn autolink(line: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut rest = line;
    while let Some((start, end)) = next_url(rest) {
        pieces.push(Piece::Text {
            text: rest[..start].to_owned(),
        });
        let url = &rest[start..end];
        pieces.push(Piece::Link {
            text: url.to_owned(),
            url: url.to_owned(),
        });
        rest = &rest[end..];
    }
    pieces.push(Piece::Text {
        text: rest.to_owned(),
    });
    pieces
}

/// The byte range of the first allowed URL in `text` that starts a word.
fn next_url(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    loop {
        let start = ["http://", "https://", "mailto:"]
            .iter()
            .filter_map(|scheme| lower[from..].find(scheme).map(|at| from + at))
            .min()?;
        let starts_word = text[..start]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric());
        let end = start + url_length(&text[start..]);
        if starts_word && link_allowed(&text[start..end]) {
            return Some((start, end));
        }
        from = start + 1;
    }
}

/// How far a URL at the start of `text` reaches: up to whitespace or a
/// character that can't be in it, without trailing punctuation.
fn url_length(text: &str) -> usize {
    let mut end = text
        .find(|c: char| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"'))
        .unwrap_or(text.len());
    while let Some(last) = text[..end].chars().next_back() {
        let unbalanced_paren =
            last == ')' && text[..end].matches('(').count() < text[..end].matches(')').count();
        if matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '\'' | ']' | '}') || unbalanced_paren
        {
            end -= last.len_utf8();
        } else {
            break;
        }
    }
    end
}

/// Reads HTML leniently into text and links. Whitespace collapses as in a
/// browser; `<br>`, `<p>` and `</p>` make line breaks.
fn html_to_raw(html: &str) -> Vec<Raw> {
    let mut raw = Vec::new();
    let mut text = String::new();
    // The open `<a>`: its allowed URL, if any, and the text so far.
    let mut link: Option<(Option<String>, String)> = None;
    let mut rest = html;

    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        if let Some(tag) = tag_at(rest) {
            rest = &rest[tag.length..];
            match (tag.name.as_str(), tag.closing) {
                ("br", _) | ("p", _) => match &mut link {
                    Some((_, link_text)) => link_text.push(' '),
                    None => text.push('\n'),
                },
                ("a", false) if link.is_none() => {
                    flush_text(&mut raw, &mut text);
                    let url = tag.href.filter(|url| link_allowed(url));
                    link = Some((url, String::new()));
                }
                ("a", true) => {
                    if let Some((url, link_text)) = link.take() {
                        let link_text = collapse_whitespace(&link_text).trim().to_owned();
                        match url {
                            Some(url) if link_text.is_empty() => raw.push(Raw::Link {
                                text: url.clone(),
                                url,
                            }),
                            Some(url) => raw.push(Raw::Link {
                                text: link_text,
                                url,
                            }),
                            None => raw.push(Raw::Text(link_text)),
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        let first = rest.chars().next().map_or(1, char::len_utf8);
        let next = rest[first..].find('<').map_or(rest.len(), |at| at + first);
        // A line break in the source is whitespace; only tags break lines.
        let decoded = decode_entities(&rest[..next]).replace(['\n', '\r'], " ");
        match &mut link {
            Some((_, link_text)) => link_text.push_str(&decoded),
            None => text.push_str(&decoded),
        }
        rest = &rest[next..];
    }
    if let Some((_, link_text)) = link {
        text.push_str(&link_text);
    }
    flush_text(&mut raw, &mut text);
    tidy_lines(raw)
}

fn flush_text(raw: &mut Vec<Raw>, text: &mut String) {
    if !text.is_empty() {
        raw.push(Raw::Text(collapse_whitespace(text)));
        text.clear();
    }
}

/// Collapses runs of spaces, tabs and source line breaks into one space, as
/// HTML does. Line breaks from tags are `\n` and stay.
fn collapse_whitespace(text: &str) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if c == '\n' {
            collapsed.push('\n');
            in_space = false;
        } else if c.is_whitespace() || c == '\r' {
            if !in_space {
                collapsed.push(' ');
            }
            in_space = true;
        } else {
            collapsed.push(c);
            in_space = false;
        }
    }
    collapsed
}

/// Trims spaces at the start and end of every line, and drops empty lines at
/// the start and end of the description.
fn tidy_lines(raw: Vec<Raw>) -> Vec<Raw> {
    let mut lines: Vec<Vec<Raw>> = vec![Vec::new()];
    for piece in raw {
        match piece {
            Raw::Text(text) => {
                for (index, part) in text.split('\n').enumerate() {
                    if index > 0 {
                        lines.push(Vec::new());
                    }
                    lines
                        .last_mut()
                        .expect("never empty")
                        .push(Raw::Text(part.to_owned()));
                }
            }
            link => lines.last_mut().expect("never empty").push(link),
        }
    }
    for line in &mut lines {
        if let Some(Raw::Text(first)) = line.first_mut() {
            *first = first.trim_start().to_owned();
        }
        if let Some(Raw::Text(last)) = line.last_mut() {
            *last = last.trim_end().to_owned();
        }
    }
    let is_blank = |line: &Vec<Raw>| {
        line.iter()
            .all(|piece| matches!(piece, Raw::Text(text) if text.is_empty()))
    };
    let first = lines.iter().position(|line| !is_blank(line));
    let last = lines.iter().rposition(|line| !is_blank(line));
    let (Some(first), Some(last)) = (first, last) else {
        return Vec::new();
    };

    let mut tidied = Vec::new();
    for (index, line) in lines.into_iter().enumerate().take(last + 1).skip(first) {
        if index > first {
            tidied.push(Raw::Text("\n".to_owned()));
        }
        tidied.extend(line);
    }
    tidied
}

struct Tag {
    name: String,
    closing: bool,
    href: Option<String>,
    /// Bytes from `<` to `>`, both included.
    length: usize,
}

/// The tag at the start of `text`, if it starts with one. A `<` that doesn't
/// start a tag, or a tag that never ends, is text.
fn tag_at(text: &str) -> Option<Tag> {
    let inner = text.strip_prefix('<')?;
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    if !inner.starts_with(|c: char| c.is_ascii_alphabetic() || c == '!' || c == '?') {
        return None;
    }
    let end = tag_end(inner)?;
    let body = &inner[..end];
    let name_length = body
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(body.len());
    Some(Tag {
        name: body[..name_length].to_ascii_lowercase(),
        closing,
        href: attribute(&body[name_length..], "href"),
        length: text.len() - inner.len() + end + 1,
    })
}

/// The position of the `>` that ends a tag, skipping quoted attribute values.
fn tag_end(inner: &str) -> Option<usize> {
    let mut quote = None;
    for (at, c) in inner.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            (None, '>') => return Some(at),
            _ => {}
        }
    }
    None
}

/// An attribute's value, with entities decoded and spaces trimmed.
fn attribute(attributes: &str, wanted: &str) -> Option<String> {
    let mut rest = attributes;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if rest.is_empty() {
            return None;
        }
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '=' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = rest[name_end..].trim_start();
        let value = if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (value, remaining) = match after.chars().next() {
                Some(quote @ ('"' | '\'')) => {
                    let body = &after[1..];
                    let end = body.find(quote).unwrap_or(body.len());
                    (&body[..end], body.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    (&after[..end], &after[end..])
                }
            };
            rest = remaining;
            value
        } else {
            ""
        };
        if name == wanted {
            return Some(decode_entities(value).trim().to_owned());
        }
    }
}

/// Decodes the character references that occur in descriptions. Unknown ones
/// stay as they are.
fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        decoded.push_str(&rest[..at]);
        rest = &rest[at..];
        let reference = rest[1..]
            .find(';')
            .filter(|&end| end <= 10)
            .and_then(|end| Some((entity(&rest[1..=end])?, end + 2)));
        match reference {
            Some((c, length)) => {
                decoded.push(c);
                rest = &rest[length..];
            }
            None => {
                decoded.push('&');
                rest = &rest[1..];
            }
        }
    }
    decoded.push_str(rest);
    decoded
}

fn entity(name: &str) -> Option<char> {
    let number = |digits: &str, radix| u32::from_str_radix(digits, radix).ok();
    let code = if let Some(hex) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
        number(hex, 16)?
    } else if let Some(decimal) = name.strip_prefix('#') {
        number(decimal, 10)?
    } else {
        return match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => None,
        };
    };
    char::from_u32(code).filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str) -> Piece {
        Piece::Text {
            text: text.to_owned(),
        }
    }

    fn link(text: &str, url: &str) -> Piece {
        Piece::Link {
            text: text.to_owned(),
            url: url.to_owned(),
        }
    }

    #[test]
    fn plain_text_becomes_one_paragraph_per_line() {
        let description = Description::from_plain_text("First line\r\nSecond\n\nFourth");

        assert_eq!(
            description.paragraphs,
            vec![
                vec![text("First line")],
                vec![text("Second")],
                vec![],
                vec![text("Fourth")]
            ]
        );
        assert_eq!(description.text, "First line\nSecond\n\nFourth");
    }

    #[test]
    fn an_empty_description_has_no_paragraphs() {
        assert!(Description::from_plain_text("").paragraphs.is_empty());
        assert!(Description::from_html("<p></p><br>").paragraphs.is_empty());
    }

    #[test]
    fn markup_in_plain_text_stays_text() {
        let description = Description::from_plain_text("Use <b>bold</b> & <Tab>");

        assert_eq!(
            description.paragraphs,
            vec![vec![text("Use <b>bold</b> & <Tab>")]]
        );
    }

    #[test]
    fn urls_in_text_become_links_without_trailing_punctuation() {
        let description = Description::from_plain_text(
            "See https://example.com/a?b=1. Or (http://example.org/x_(y)), mail mailto:me@example.com!",
        );

        assert_eq!(
            description.paragraphs,
            vec![vec![
                text("See "),
                link("https://example.com/a?b=1", "https://example.com/a?b=1"),
                text(". Or ("),
                link("http://example.org/x_(y)", "http://example.org/x_(y)"),
                text("), mail "),
                link("mailto:me@example.com", "mailto:me@example.com"),
                text("!"),
            ]]
        );
    }

    #[test]
    fn other_schemes_and_urls_inside_words_stay_text() {
        for plain in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<script>x</script>",
            "xhttps://example.com",
            "https://",
        ] {
            assert_eq!(
                Description::from_plain_text(plain).paragraphs,
                vec![vec![text(plain)]],
                "{plain:?}"
            );
        }
    }

    #[test]
    fn upper_case_schemes_are_links_too() {
        assert_eq!(
            Description::from_plain_text("HTTPS://EXAMPLE.COM").paragraphs,
            vec![vec![link("HTTPS://EXAMPLE.COM", "HTTPS://EXAMPLE.COM")]]
        );
    }

    #[test]
    fn html_line_breaks_and_paragraphs_become_lines() {
        let description =
            Description::from_html("<p>First   <b>bold</b>\n line</p><p>Second<br>Third<BR/></p>");

        assert_eq!(
            description.paragraphs,
            vec![
                vec![text("First bold line")],
                vec![],
                vec![text("Second")],
                vec![text("Third")]
            ]
        );
    }

    #[test]
    fn html_tags_are_dropped_and_their_text_kept() {
        let description = Description::from_html(
            "<img src=x onerror=alert(1)><script>alert(2)</script><!-- note --><i>kept</i>",
        );

        assert_eq!(description.paragraphs, vec![vec![text("alert(2)kept")]]);
    }

    #[test]
    fn html_entities_are_decoded_once() {
        let description =
            Description::from_html("a &amp; b &lt;b&gt; &amp;lt; &#x41;&#66; &bogus;");

        assert_eq!(
            description.paragraphs,
            vec![vec![text("a & b <b> &lt; AB &bogus;")]]
        );
    }

    #[test]
    fn html_links_keep_their_text_and_only_allowed_urls() {
        let description = Description::from_html(
            "<a href=\"https://example.com/a?x=1&amp;y=2\">the agenda</a> \
             <a href='javascript:alert(1)'>click me</a> \
             <a href=mailto:me@example.com></a> \
             <a href=\"file:///etc/passwd\">https://example.com/shown</a>",
        );

        assert_eq!(
            description.paragraphs,
            vec![vec![
                link("the agenda", "https://example.com/a?x=1&y=2"),
                text(" click me "),
                link("mailto:me@example.com", "mailto:me@example.com"),
                text(" "),
                link("https://example.com/shown", "https://example.com/shown"),
            ]]
        );
    }

    #[test]
    fn the_plain_text_form_keeps_link_urls() {
        let description = Description::from_html(
            "Agenda: <a href=\"https://example.com/doc\">the doc</a><br>Bye",
        );

        assert_eq!(
            description.text,
            "Agenda: the doc (https://example.com/doc)\nBye"
        );
    }

    #[test]
    fn unfinished_markup_is_text() {
        assert_eq!(
            Description::from_html("a < b and <a href=\"x").paragraphs,
            vec![vec![text("a < b and <a href=\"x")]]
        );
    }

    #[test]
    fn only_http_https_and_mailto_links_are_allowed() {
        for allowed in [
            "http://a",
            "https://example.com/x",
            "mailto:me@example.com",
            "HTTP://A",
        ] {
            assert!(link_allowed(allowed), "{allowed:?}");
        }
        for refused in [
            "",
            "https://",
            "mailto:",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,x",
            "ftp://example.com",
            "https://example.com/a b",
            "https://example.com/\nx",
            " https://example.com",
        ] {
            assert!(!link_allowed(refused), "{refused:?}");
        }
    }
}
