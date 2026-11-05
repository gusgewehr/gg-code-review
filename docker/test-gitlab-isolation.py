"""Static checks: runner isolation, pinned glab checksums, no TLS-skip options."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AMD = "bc024819e7598b19429a199cd87f640d98b2b67416d8ad796f19e4260b57a7c7"
ARM = "60d0512085a8c1be7c5ef51aa7e4ee28213444cea0376b3260707d90489199ea"
FORBIDDEN_RUNNER = ["OCR_GITLAB", "ocr-ca", "GLAB", "GITLAB_TOKEN"]
FORBIDDEN_TLS = ["GIT_SSL_NO_VERIFY", "skip_tls_verify", "skip-tls-verify", "insecure-skip-tls-verify",
                 "danger_accept_invalid_certs", "sslVerify"]


def fail(msg):
    print("FAIL: " + msg)
    sys.exit(1)


def text(rel):
    return (ROOT / rel).read_text(encoding="utf-8", errors="replace")


def blocks(compose):
    """Map top-level key and two-space-indented key to its text."""
    out, key, buf = {}, None, []
    for line in compose.splitlines():
        m = re.match(r"^(?:  )?(x-review-runner|[a-z][\w-]*):", line)
        if m and (line.startswith("  ") or line.startswith("x-")):
            if key:
                out[key] = "\n".join(buf)
            key, buf = m.group(1), []
        elif line and not line.startswith(" ") and not line.startswith("#"):
            if key:
                out[key] = "\n".join(buf)
            key, buf = None, []
        if key:
            buf.append(line)
    if key:
        out[key] = "\n".join(buf)
    return out


def check_dockerfile():
    d = text("docker/Dockerfile")
    for needle in ("GLAB_VERSION=1.128.0", AMD, ARM, "sha256sum -c"):
        if needle not in d:
            fail("docker/Dockerfile missing " + needle)
    print("ok dockerfile glab pinned")


def check_runner_image():
    r = text("docker/Dockerfile.runner").lower()
    if "glab" in r or "/run/ocr-ca" in r:
        fail("Dockerfile.runner mentions glab or the CA mount")
    print("ok runner image clean")


def check_compose():
    b = blocks(text("docker-compose.yml"))
    for k in ("review-runner", "review-egress", "x-review-runner"):
        if k not in b:
            fail("compose block not found: " + k)
        for bad in FORBIDDEN_RUNNER:
            if bad in b[k]:
                fail("%s contains %s" % (k, bad))
    for needle in ("OCR_FORGE", "/run/ocr-ca/ca.crt:ro"):
        if needle not in b.get("server", ""):
            fail("server block missing " + needle)
    print("ok compose isolation")


def check_no_skip_verify():
    files = [p for p in (ROOT / "backend/src").rglob("*") if p.is_file()]
    files += [p for p in (ROOT / "docker").rglob("*") if p.is_file() and p.resolve() != Path(__file__).resolve()]
    files += list(ROOT.glob("docker-compose*.yml"))
    for p in files:
        try:
            t = p.read_text(encoding="utf-8").lower()
        except (UnicodeDecodeError, OSError):
            continue
        for bad in FORBIDDEN_TLS:
            if bad.lower() in t:
                fail("%s contains %s" % (p.relative_to(ROOT), bad))
    print("ok no tls skip options")


INSTRUCTIONS = {"FROM", "RUN", "CMD", "LABEL", "EXPOSE", "ENV", "ADD", "COPY", "ENTRYPOINT", "VOLUME", "USER",
                "WORKDIR", "ARG", "ONBUILD", "STOPSIGNAL", "HEALTHCHECK", "SHELL", "MAINTAINER"}


def check_dockerfile_continuations():
    # Line-based: sound only while no Dockerfile uses heredocs.
    for p in sorted((ROOT / "docker").glob("Dockerfile*")):
        cont = False
        for n, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            s = line.strip()
            if not s or s.startswith("#"):
                continue
            if not cont and s.split()[0].upper() not in INSTRUCTIONS:
                fail("%s:%d is not an instruction or continuation" % (p.relative_to(ROOT), n))
            cont = line.rstrip().endswith("\\")
    print("ok dockerfile continuations")


for fn in (check_dockerfile, check_dockerfile_continuations, check_runner_image, check_compose, check_no_skip_verify):
    fn()
