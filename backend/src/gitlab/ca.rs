//! Internal CA trust bundle for server child processes (git, glab, outbound TLS).

use std::path::{Path, PathBuf};

const CA_PATH: &str = "/run/ocr-ca/ca.crt";
const SYSTEM_BUNDLE: &str = "/etc/ssl/certs/ca-certificates.crt";
const BUNDLE_NAME: &str = "ca-bundle.crt";

pub(crate) fn build_bundle(system: &str, extra: &str) -> Result<String, String> {
    if !extra.contains("-----BEGIN CERTIFICATE-----") {
        return Err(format!("{CA_PATH} does not contain a PEM certificate."));
    }
    Ok(format!("{system}\n{extra}"))
}

fn prepare_at(ca: &Path, system: &Path, data_dir: &Path) -> Result<Option<PathBuf>, String> {
    if ca.is_dir() {
        return Err(format!(
            "{} is a directory: set OCR_GITLAB_CA_FILE to the CA certificate file (Docker created a directory because the host file was missing).",
            ca.display()
        ));
    }
    let Ok(extra) = std::fs::read_to_string(ca) else {
        return Ok(None);
    };
    if extra.trim().is_empty() {
        return Ok(None);
    }
    let system_pem = std::fs::read_to_string(system)
        .map_err(|error| format!("could not read system CA bundle {}: {error}", system.display()))?;
    let bundle = build_bundle(&system_pem, &extra)?;
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let target = data_dir.join(BUNDLE_NAME);
    let tmp = data_dir.join(format!("{BUNDLE_NAME}.tmp"));
    std::fs::write(&tmp, bundle).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&tmp, &target).map_err(|error| error.to_string())?;
    Ok(Some(target))
}

/// Must run before any thread starts: it mutates the process environment.
pub(crate) fn prepare(data_dir: &Path) -> Result<(), String> {
    if let Some(bundle) = prepare_at(Path::new(CA_PATH), Path::new(SYSTEM_BUNDLE), data_dir)? {
        for key in ["SSL_CERT_FILE", "GIT_SSL_CAINFO", "GLAB_CA_CERT"] {
            std::env::set_var(key, &bundle);
        }
        println!("Trusting internal CA from {CA_PATH}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEM: &str = "-----BEGIN CERTIFICATE-----\nX\n-----END CERTIFICATE-----\n";

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ocr-ca-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn build_bundle_appends_and_validates() {
        let out = build_bundle("SYS\n", PEM).unwrap();
        assert!(out.starts_with("SYS") && out.contains(&format!("\n{PEM}")));
        let err = build_bundle("SYS", "not a cert").unwrap_err();
        assert!(err.contains("/run/ocr-ca/ca.crt") && err.contains("PEM"));
    }

    #[test]
    fn prepare_at_cases() {
        let dir = tmp("cases");
        let data = dir.join("data");
        assert_eq!(prepare_at(&dir.join("missing"), &dir, &data).unwrap(), None);
        let empty = dir.join("empty.crt");
        std::fs::write(&empty, "\n").unwrap();
        assert_eq!(prepare_at(&empty, &dir, &data).unwrap(), None);
        assert!(!data.exists());
        assert!(prepare_at(&dir, &dir, &data)
            .unwrap_err()
            .contains("OCR_GITLAB_CA_FILE"));
        let ca = dir.join("ca.crt");
        std::fs::write(&ca, PEM).unwrap();
        let sys = dir.join("sys.crt");
        std::fs::write(&sys, "SYS\n").unwrap();
        let out = prepare_at(&ca, &sys, &data).unwrap().unwrap();
        assert_eq!(out, data.join("ca-bundle.crt"));
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("SYS") && text.contains(PEM));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&out).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
