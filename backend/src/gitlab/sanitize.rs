//! Neutralises model text before it becomes a GitLab note: no quick actions, no mentions or cross references,
//! no applicable suggestions, and a hard size cap. shortcut: a full http(s) URL to an issue/MR on the same
//! instance is left intact (rewriting it would break the link) and GitLab may still cross-reference it.
//! Only a line that is exactly one HTML comment (the OCR
//! markers) is left untouched. Code spans and fences are neutralised too: the sanitizer cannot know for sure
//! what GitLab renders as code (unbalanced backticks, HTML blocks, list containers), so it never guesses.

pub(super) const ZWSP: char = '\u{200B}';
pub(super) const MAX_BODY_CHARS: usize = 1_000_000;
const NOTICE: &str = "\n\n_Truncated by OpenCodeReview: the original text exceeded GitLab's note size limit._\n\n";
/// Characters that always end an http(s) link in GitLab's Markdown (link syntax, code, tables, escapes, math).
const URL_END: &[char] = &['(', ')', '[', ']', '{', '}', '<', '>', '"', '`', '\\', '|', '$'];
/// Emphasis/strikethrough delimiters: they end a link only when an opener was seen earlier in the body.
const EMPHASIS: &[char] = &['*', '~'];

/// Removes every zero-width space so stored bodies can be matched against markers, fingerprints and titles.
pub(super) fn strip_zwsp(body: &str) -> String {
    body.replace(ZWSP, "")
}

/// A whole line that is one well-formed HTML comment: GitLab renders nothing of it.
fn is_marker(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 7 || !t.starts_with("<!--") || !t.ends_with("-->") {
        return false;
    }
    let inner = &t[4..t.len() - 3];
    !inner.contains("--") && !inner.starts_with('>') && !inner.starts_with("->")
}

/// Whether `token[at]` ends a link: a hard end, a `_` that could close emphasis (not intraword), or an
/// emphasis delimiter with an opener earlier in the body (`opened`) or the token.
fn ends_url(token: &str, at: usize, c: char, opened: &[char]) -> bool {
    let next = token[at + c.len_utf8()..].chars().next();
    URL_END.contains(&c)
        || (c == '_' && !next.is_some_and(|n| n.is_ascii_alphanumeric()))
        || (EMPHASIS.contains(&c) && (opened.contains(&c) || token[..at].contains(c)))
}

/// Byte ranges of the http(s) links GitLab autolinks in a whitespace-free token, each ending before the
/// first character that ends it ([`ends_url`]). A scheme glued to other text is not a link, so it gets no
/// exemption.
fn url_spans(token: &str, opened: &[char]) -> Vec<(usize, usize)> {
    let lower = token.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some((start, scheme)) = ["https://", "http://"]
        .iter()
        .filter_map(|s| lower[from..].find(s).map(|i| (from + i, s.len())))
        .min()
    {
        let body = start + scheme;
        let end = token[body..]
            .char_indices()
            .find(|(i, c)| ends_url(token, body + i, *c, opened))
            .map_or(token.len(), |(i, _)| body + i);
        let glued = token[..start]
            .chars()
            .next_back()
            .is_some_and(|c| !matches!(c, '(' | '<' | '*' | '_' | '~'));
        if !glued {
            spans.push((start, end));
        }
        from = end;
    }
    spans
}

/// Whether GitLab could read `chars[i]` as the sigil of a mention or reference (cross-project ones included).
fn is_live_sigil(chars: &[(usize, char)], i: usize) -> bool {
    let prev = i.checked_sub(1).map(|j| chars[j].1);
    let next = chars.get(i + 1).map(|(_, c)| *c);
    let word = next.is_some_and(|n| n.is_ascii_alphanumeric() || n == '_');
    match chars[i].1 {
        // `project@<sha>` / `group/project@<sha>` is a cross-project commit reference.
        '@' => word && (!prev.is_some_and(|p| p.is_ascii_alphanumeric()) || is_sha_ref(chars, i)),
        '#' | '!' | '$' => next.is_some_and(|n| n.is_ascii_digit()),
        // An HTML entity (`&commat;`, `&#x40;`) would decode into a live sigil.
        '&' => next.is_some_and(|n| n.is_ascii_digit()) || is_entity(chars, i),
        '~' | '%' => word || next == Some('"'),
        _ => false,
    }
}

fn is_sha_ref(chars: &[(usize, char)], i: usize) -> bool {
    let hex = chars[i + 1..].iter().take_while(|(_, c)| c.is_ascii_hexdigit()).count();
    hex >= 7 && !chars.get(i + 1 + hex).is_some_and(|(_, c)| c.is_ascii_alphanumeric())
}

fn is_entity(chars: &[(usize, char)], i: usize) -> bool {
    let name = chars[i + 1..]
        .iter()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '#')
        .count();
    name > 0 && chars.get(i + 1 + name).is_some_and(|(_, c)| *c == ';')
}

fn neutralize_token(token: &str, opened: &mut Vec<char>, out: &mut String) {
    let spans = url_spans(token, opened);
    let chars: Vec<(usize, char)> = token.char_indices().collect();
    for (i, (at, c)) in chars.iter().enumerate() {
        out.push(*c);
        if !spans.iter().any(|(s, e)| (*s..*e).contains(at)) && is_live_sigil(&chars, i) {
            out.push(ZWSP);
        }
    }
    opened.extend(EMPHASIS.iter().filter(|c| token.contains(**c)));
}

fn neutralize_line(line: &str, opened: &mut Vec<char>) -> String {
    let mut out = String::with_capacity(line.len() + 4);
    let mut token = String::new();
    for c in line.chars() {
        if c.is_whitespace() {
            neutralize_token(&token, opened, &mut out);
            token.clear();
            out.push(c);
        } else {
            token.push(c);
        }
    }
    neutralize_token(&token, opened, &mut out);
    out
}

fn defuse_slash(line: &str) -> String {
    let rest = line.trim_start();
    let at = line.len() - rest.len();
    let mut chars = rest.chars();
    if chars.next() == Some('/') && chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return format!("{}{ZWSP}{rest}", &line[..at]);
    }
    line.to_string()
}

/// Any fence run with a `suggestion` info string becomes `diff`, wherever it sits on the line (blockquote and
/// list containers included) and whatever the fence state.
fn defuse_suggestion(line: String) -> String {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let run = bytes[i..].iter().take_while(|b| **b == bytes[i]).count();
        if matches!(bytes[i], b'`' | b'~') && run >= 3 {
            let info = line[i + run..].trim_start_matches([' ', '\t']);
            if info.get(..10).is_some_and(|w| w.eq_ignore_ascii_case("suggestion")) {
                return format!("{}diff", &line[..i + run]);
            }
        }
        i += run;
    }
    line
}

pub(super) fn finalize_body(body: &str) -> String {
    let mut opened = Vec::new();
    let lines = body
        .split('\n')
        .map(|line| {
            if is_marker(line) {
                line.to_string()
            } else {
                neutralize_line(&defuse_suggestion(defuse_slash(line)), &mut opened)
            }
        })
        .collect();
    truncate(lines)
}

fn truncate(lines: Vec<String>) -> String {
    let joined = lines.join("\n");
    if joined.chars().count() <= MAX_BODY_CHARS {
        return joined;
    }
    let keep = lines.len() - lines.iter().rev().take_while(|l| is_marker(l)).count();
    let tail = lines[keep..].join("\n");
    let head = lines[..keep].join("\n");
    let budget = MAX_BODY_CHARS.saturating_sub(NOTICE.chars().count() + tail.chars().count());
    eprintln!("gitlab publish: body truncated to the note size limit");
    let prefix: String = head.chars().take(budget).collect();
    format!("{prefix}{NOTICE}{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(s: &str) -> String {
        s.replace('^', &ZWSP.to_string())
    }

    #[test]
    fn sanitize_slash_command_line() {
        assert_eq!(finalize_body("/merge"), z("^/merge"));
        assert_eq!(finalize_body("a\n/approve now"), z("a\n^/approve now"));
    }

    #[test]
    fn sanitize_indented_slash_command() {
        assert_eq!(finalize_body("  /approve"), z("  ^/approve"));
    }

    #[test]
    fn sanitize_slash_non_commands_unchanged() {
        assert_eq!(finalize_body("// note"), "// note");
        assert_eq!(finalize_body("/ x"), "/ x");
    }

    #[test]
    fn sanitize_slash_inside_fence() {
        assert_eq!(finalize_body("```\n/merge\n```"), z("```\n^/merge\n```"));
    }

    #[test]
    fn sanitize_mention() {
        assert_eq!(finalize_body("hi @alice"), z("hi @^alice"));
    }

    #[test]
    fn sanitize_email_unchanged() {
        assert_eq!(finalize_body("a@b.com"), "a@b.com");
    }

    #[test]
    fn sanitize_each_sigil() {
        for (src, want) in [
            ("#12", "#^12"),
            ("!5", "!^5"),
            ("~bug", "~^bug"),
            ("%1.0", "%^1.0"),
            ("$3", "$^3"),
            ("&4", "&^4"),
        ] {
            assert_eq!(finalize_body(src), z(want), "{src}");
        }
    }

    #[test]
    fn sanitize_plain_sigils_unchanged() {
        assert_eq!(finalize_body("50% done"), "50% done");
        assert_eq!(finalize_body("### Title"), "### Title");
        assert_eq!(
            finalize_body("&mut self, #include, !flag"),
            "&mut self, #include, !flag"
        );
    }

    #[test]
    fn sanitize_urls_unchanged() {
        for s in [
            "https://h/g/p/-/commit/abc#L1",
            "[x](https://h/a#b)",
            "<https://h/@alice>",
            "https://h/a#12",
            "https://a/b,@alice",
            "https://h/acme_web/x%20y/@ui/pkg!1",
            "https://h/~user/a*b@c",
        ] {
            assert_eq!(finalize_body(s), s);
        }
    }

    #[test]
    fn sanitize_code_is_neutralised_too() {
        assert_eq!(finalize_body("`@alice #1` and @bob"), z("`@^alice #^1` and @^bob"));
        assert_eq!(finalize_body("~~~\n@a #1\n~~~\n@b"), z("~~~\n@^a #^1\n~~~\n@^b"));
    }

    #[test]
    fn sanitize_bypasses_are_closed() {
        for (src, want) in [
            ("<!-- -->@all", "<!-- -->@^all"),
            ("<!-- a --> @all <!-- b -->", "<!-- a --> @^all <!-- b -->"),
            ("<!-- a --!> @all -->", "<!-- a --!> @^all -->"),
            ("a ` @all", "a ` @^all"),
            ("`x`` @all `y`", "`x`` @^all `y`"),
            ("[x](https://h)@all", "[x](https://h)@^all"),
            ("*x* https://h*@all", "*x* https://h*@^all"),
            ("https://h_@all", "https://h_@^all"),
            ("g/p@abc1234 p@deadbeef0", "g/p@^abc1234 p@^deadbeef0"),
            ("xhttps://h/@all", "xhttps://h/@^all"),
            ("g/p#1", "g/p#^1"),
            ("project!3", "project!^3"),
            ("~\"x y\"", "~^\"x y\""),
            ("%\"v1.0\"", "%^\"v1.0\""),
            ("&commat;all &#x40;all", "&^commat;all &^#x40;all"),
            ("<div>\n```\n</div>\n\n@all", "<div>\n```\n</div>\n\n@^all"),
        ] {
            assert_eq!(finalize_body(src), z(want), "{src}");
        }
    }

    #[test]
    fn sanitize_marker_lines_untouched() {
        let m = "<!-- ocr:fingerprint=fnv64:abc @x #1 -->\n  <!-- ocr-review-post:v1 /merge -->";
        assert_eq!(finalize_body(m), m);
    }

    #[test]
    fn sanitize_suggestion_fence_becomes_diff() {
        assert_eq!(finalize_body("```suggestion\n-a\n+b\n```"), "```diff\n-a\n+b\n```");
        assert_eq!(finalize_body("```suggestion:-0+0\nx\n```"), "```diff\nx\n```");
        assert_eq!(finalize_body("```rust\nx\n```"), "```rust\nx\n```");
        assert_eq!(finalize_body("~~~\n```suggestion\n~~~"), "~~~\n```diff\n~~~");
        assert_eq!(finalize_body("> ```suggestion\n> x"), "> ```diff\n> x");
        assert_eq!(finalize_body("- ``` Suggestion:-1+0"), "- ```diff");
    }

    #[test]
    fn sanitize_truncates_on_char_boundary_keeping_markers() {
        let marker = "<!-- ocr-review-post:v1 abc -->";
        let body = format!("{}\n{marker}", "é".repeat(MAX_BODY_CHARS + 1));
        let out = finalize_body(&body);
        assert!(out.chars().count() <= MAX_BODY_CHARS);
        assert!(out.ends_with(marker));
        assert!(out.contains("Truncated by OpenCodeReview"));
    }

    #[test]
    fn sanitize_round_trips_through_strip() {
        let src = "/merge @a #1\n```\n/x @y\n```\n<!-- m -->\nplain `c` https://h/@z";
        assert_eq!(strip_zwsp(&finalize_body(src)), src);
    }
}
