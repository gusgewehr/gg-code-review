//! Pins the forge provider and host of a deploy in the settings table.

use super::{provider_name, Forge};

pub(crate) fn check(stored: (&str, &str), configured: (&str, &str)) -> Result<(), String> {
    if stored == configured {
        return Ok(());
    }
    Err(format!(
        "This deploy is pinned to forge {} on host {}, but the configuration says forge {} on host {}. \
Restore the previous OCR_FORGE / OCR_GITLAB_HOST / OCR_GH_HOST values, or use a fresh database: \
changing the forge or host over existing data is not supported.",
        stored.0, stored.1, configured.0, configured.1
    ))
}

/// Record the forge on first boot, refuse a different one afterwards.
/// `host` None (GitHub host not detectable) records nothing and skips the guard.
pub(crate) fn enforce(db: &crate::db::Database, forge: Forge, host: Option<&str>) -> Result<(), String> {
    let Some(host) = host else { return Ok(()) };
    let provider = provider_name(forge);
    db.ensure_setting_default("forge_provider", provider)?;
    db.ensure_setting_default("forge_host", host)?;
    let stored_provider = db.setting("forge_provider")?.unwrap_or_default();
    let stored_host = db.setting("forge_host")?.unwrap_or_default();
    check((&stored_provider, &stored_host), (provider, host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_check_names_changed_values() {
        assert!(check(("github", "github.com"), ("github", "github.com")).is_ok());
        let err = check(("github", "github.com"), ("gitlab", "gitlab.x")).unwrap_err();
        assert!(err.contains("github") && err.contains("gitlab") && err.contains("Restore"));
        let err = check(("gitlab", "a.x"), ("gitlab", "b.x")).unwrap_err();
        assert!(err.contains("a.x") && err.contains("b.x"));
    }

    #[test]
    fn guard_enforce_pins_and_rejects_change() {
        let db = crate::db::Db::for_test().open().unwrap();
        enforce(&db, Forge::GitHub, Some("github.com")).unwrap();
        assert_eq!(db.setting("forge_provider").unwrap().as_deref(), Some("github"));
        enforce(&db, Forge::GitHub, Some("github.com")).unwrap();
        assert!(enforce(&db, Forge::GitLab, Some("gitlab.x")).is_err());
        assert!(db.save_setting("forge_provider", "gitlab").is_err());
    }

    #[test]
    fn guard_skipped_without_host() {
        let db = crate::db::Db::for_test().open().unwrap();
        enforce(&db, Forge::GitHub, None).unwrap();
        assert_eq!(db.setting("forge_provider").unwrap(), None);
    }
}
