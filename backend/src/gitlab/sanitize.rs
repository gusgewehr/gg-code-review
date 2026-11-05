//! Neutralises model text before it becomes a GitLab note: no quick actions, no mentions or cross references,
//! no applicable suggestions, and a hard size cap. Marker lines (`<!-- ... -->`) are never touched.

pub(super) const ZWSP: char = '\u{200B}';
pub(super) const MAX_BODY_CHARS: usize = 1_000_000;
const NOTICE: &str = "\n\n_Truncated by OpenCodeReview: the original text exceeded GitLab's note size limit._\n\n";

/// Removes every zero-width space so stored bodies can be matched against markers, fingerprints and titles.
pub(super) fn strip_zwsp(body: &str) -> String {
    body.replace(ZWSP, "")
}

fn is_marker(line: &str) -> bool {
    line.trim_start().starts_with("<!--")
}

/// (indent, fence char, run length, info string) of a fence line, if it is one.
fn fence_of(line: &str) -> Option<(usize, char, usize, &str)> {
    let rest = line.trim_start_matches(' ');
    let indent = line.len() - rest.len();
    let ch = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = rest.chars().take_while(|c| *c == ch).count();
    let info = &rest[run..];
    if indent > 3 || run < 3 || (ch == '`' && info.contains('`')) {
        return None;
    }
    Some((indent, ch, run, info))
}

fn neutralize_token(token: &str, out: &mut String) {
    if token.contains("://") {
        out.push_str(token);
        return;
    }
    let chars: Vec<char> = token.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        out.push(*c);
        if matches!(c, '@' | '#' | '!' | '~' | '%' | '$' | '&')
            && (i == 0 || !chars[i - 1].is_ascii_alphanumeric())
            && chars.get(i + 1).is_some_and(|n| n.is_ascii_alphanumeric() || *n == '_')
        {
            out.push(ZWSP);
        }
    }
}

fn neutralize_text(segment: &str, out: &mut String) {
    let mut token = String::new();
    for c in segment.chars() {
        if c.is_whitespace() {
            neutralize_token(&token, out);
            token.clear();
            out.push(c);
        } else {
            token.push(c);
        }
    }
    neutralize_token(&token, out);
}

fn neutralize_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 4);
    for (i, segment) in line.split('`').enumerate() {
        if i > 0 {
            out.push('`');
        }
        if i % 2 == 0 {
            neutralize_text(segment, &mut out);
        } else {
            out.push_str(segment);
        }
    }
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

pub(super) fn finalize_body(body: &str) -> String {
    let mut open: Option<(char, usize)> = None;
    let mut lines: Vec<String> = Vec::new();
    for line in body.split('\n') {
        if is_marker(line) {
            lines.push(line.to_string());
            continue;
        }
        let line = defuse_slash(line);
        if let Some((ch, run)) = open {
            if fence_of(&line).is_some_and(|(_, c, r, info)| c == ch && r >= run && info.trim().is_empty()) {
                open = None;
            }
            lines.push(line);
        } else if let Some((indent, ch, run, info)) = fence_of(&line) {
            open = Some((ch, run));
            if info.trim_start().starts_with("suggestion") {
                lines.push(format!("{}{}diff", " ".repeat(indent), ch.to_string().repeat(run)));
            } else {
                lines.push(line);
            }
        } else {
            lines.push(neutralize_line(&line));
        }
    }
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
    }

    #[test]
    fn sanitize_urls_unchanged() {
        for s in ["https://h/g/p/-/commit/abc#L1", "[x](https://h/a#b)"] {
            assert_eq!(finalize_body(s), s);
        }
    }

    #[test]
    fn sanitize_inline_code_unchanged() {
        assert_eq!(finalize_body("`@alice #1` and @bob"), z("`@alice #1` and @^bob"));
    }

    #[test]
    fn sanitize_fence_sigils_unchanged() {
        assert_eq!(finalize_body("~~~\n@a #1\n~~~\n@b"), z("~~~\n@a #1\n~~~\n@^b"));
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
        let src = "/merge @a #1\n```\n/x\n```\n<!-- m -->\nplain `c`";
        assert_eq!(strip_zwsp(&finalize_body(src)), src);
    }
}
