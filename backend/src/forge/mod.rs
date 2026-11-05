//! Hosting-provider seam: code outside `github/` goes through here.

#[allow(unused_imports)]
pub(crate) use crate::github::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Forge {
    GitHub,
}

// shortcut: always GitHub; Phase 2 selects the provider from deploy config (OCR_FORGE).
pub(crate) fn current() -> Forge {
    Forge::GitHub
}

impl Forge {
    pub(crate) fn commit_url(self, full_name: &str, sha: &str) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/commit/{sha}"),
        }
    }

    pub(crate) fn review_url(self, full_name: &str, number: i64, review_id: i64) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/pull/{number}#pullrequestreview-{review_id}"),
        }
    }

    pub(crate) fn pr_head_ref(self, number: i64) -> String {
        match self {
            Forge::GitHub => format!("refs/pull/{number}/head"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_urls_and_refspec_are_unchanged() {
        assert_eq!(current(), Forge::GitHub);
        assert_eq!(Forge::GitHub.commit_url("o/r", "abc123"), "https://github.com/o/r/commit/abc123");
        assert_eq!(Forge::GitHub.review_url("o/r", 7, 42), "https://github.com/o/r/pull/7#pullrequestreview-42");
        assert_eq!(Forge::GitHub.pr_head_ref(7), "refs/pull/7/head");
    }
}
