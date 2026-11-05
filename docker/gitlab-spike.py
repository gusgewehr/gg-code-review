"""Probe a real self-hosted GitLab for the OpenCodeReview GitLab connection spike.

Usage:
  export GITLAB_TOKEN=...   (token only from env, never argv)
  python3 -I docker/gitlab-spike.py --host gitlab.example.com [--ca ca.crt] [--with-glab] [--project group/sub/proj]

Writes gitlab-spike-result.json in the current directory; all captured text is redacted.
"""
import argparse
import json
import os
import re
import shutil
import ssl
import subprocess
import sys
import tempfile
import urllib.request
from urllib.error import HTTPError

TOKEN = os.environ.get("GITLAB_TOKEN", "")
PAGE_HEADERS = ("X-Next-Page", "X-Total", "X-Total-Pages", "Link")
SAFE_KEYS = ("version", "revision", "username", "scopes", "expires_at", "active", "revoked")


def redact(text):
    text = text or ""
    if TOKEN:
        text = text.replace(TOKEN, "[redacted]")
    return re.sub(r"glpat-[A-Za-z0-9_-]+", "[redacted]", text)


def api(host, path, ctx):
    req = urllib.request.Request("https://%s/api/v4%s" % (host, path), headers={"PRIVATE-TOKEN": TOKEN})
    rec = {"path": path}
    try:
        with urllib.request.urlopen(req, context=ctx, timeout=30) as r:
            status, body, hdrs = r.status, r.read(), r.headers
    except HTTPError as e:
        status, body, hdrs = e.code, e.read(), e.headers
    except Exception as e:  # TLS, DNS, timeout: the text is the finding
        rec["error"] = redact(repr(e))
        return rec
    rec["status"] = status
    rec["pagination"] = {h: hdrs.get(h) for h in PAGE_HEADERS if hdrs.get(h)}
    try:
        data = json.loads(body)
        item = data[0] if isinstance(data, list) and data else data
        if isinstance(data, list):
            rec["items"] = len(data)
        if isinstance(item, dict):
            rec["keys"] = sorted(item.keys())
            rec["values"] = {k: item[k] for k in SAFE_KEYS if k in item}
        else:
            rec["body"] = redact(str(data)[:200])
    except ValueError:
        rec["body"] = redact(body.decode("utf-8", "replace")[:200])
    return rec


def run(cmd, env, stdin=None):
    p = subprocess.run(cmd, env=env, input=stdin, capture_output=True, text=True, timeout=120)
    return {"cmd": redact(" ".join(cmd)), "exit": p.returncode, "out": redact(p.stdout[:2000]),
            "err": redact(p.stderr[:2000])}


def ls_remote(host, project, ca):
    env = dict(os.environ, GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="http.extraHeader",
               GIT_CONFIG_VALUE_0="Authorization: Bearer " + TOKEN, GIT_TERMINAL_PROMPT="0")
    if ca:
        env["GIT_SSL_CAINFO"] = ca
    p = subprocess.run(["git", "ls-remote", "https://%s/%s.git" % (host, project), "refs/merge-requests/*/head"],
                       env=env, capture_output=True, text=True, timeout=120)
    refs = len([ln for ln in p.stdout.splitlines() if ln.strip()])
    return {"exit": p.returncode, "refs": refs, "err": redact(p.stderr[:300])}


def glab_probe(host, ca):
    d, d2 = tempfile.mkdtemp(), tempfile.mkdtemp()
    env = {k: v for k, v in os.environ.items() if k != "GITLAB_TOKEN"}
    env.update(GLAB_CONFIG_DIR=d, GLAB_SEND_TELEMETRY="false", GLAB_CHECK_UPDATE="false", NO_PROMPT="1")
    if ca:
        env["GLAB_CA_CERT"] = ca
    login = ["glab", "auth", "login", "--hostname", host, "--stdin", "--api-protocol", "https",
             "--git-protocol", "https", "--insecure-storage"]
    out = {"login": run(login, env, TOKEN)}
    out["status"] = run(["glab", "auth", "status", "--hostname", host], env)
    out["user"] = run(["glab", "api", "--hostname", host, "user"], env)
    out["pat_self"] = run(["glab", "api", "--hostname", host, "personal_access_tokens/self"], env)
    pg = run(["glab", "api", "--hostname", host, "--paginate", "projects?membership=true&per_page=1"], env)
    pg["concatenated_arrays"] = "][" in pg["out"]
    out["paginate"] = pg
    out["invalid_login"] = run(login, dict(env, GLAB_CONFIG_DIR=d2), "glpat-invalidinvalidinvalid\n")
    cfg = os.path.join(d, "config.yml")
    if os.path.exists(cfg):
        with open(cfg, encoding="utf-8") as f:
            out["config_layout"] = re.sub(r"(token:\s*).*", r"\1[masked]", f.read())
    if ca:
        env3 = dict(env, SSL_CERT_FILE=ca)
        env3.pop("GLAB_CA_CERT", None)
        out["ssl_cert_file_only"] = run(["glab", "api", "--hostname", host, "user"], env3)
    shutil.rmtree(d, ignore_errors=True)
    shutil.rmtree(d2, ignore_errors=True)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--host", required=True)
    ap.add_argument("--ca")
    ap.add_argument("--with-glab", action="store_true")
    ap.add_argument("--project")
    a = ap.parse_args()
    if not TOKEN:
        sys.exit("GITLAB_TOKEN env var is required")
    res = {"token_shape": {"prefix": TOKEN.split("-")[0], "length": len(TOKEN)}, "host": a.host}
    res["default_ca_probe"] = api(a.host, "/version", ssl.create_default_context())
    ctx = ssl.create_default_context(cafile=a.ca) if a.ca else ssl.create_default_context()
    paths = ["/version", "/user", "/personal_access_tokens/self", "/projects?membership=true&per_page=1",
             "/merge_requests?state=opened&scope=all&per_page=1"]
    res["api"] = [api(a.host, p, ctx) for p in paths]
    if a.project:
        res["ls_remote"] = ls_remote(a.host, a.project, a.ca)
    if a.with_glab:
        res["glab"] = glab_probe(a.host, a.ca)
    with open("gitlab-spike-result.json", "w", encoding="utf-8") as f:
        json.dump(json.loads(redact(json.dumps(res))), f, indent=2)
    for r in res["api"]:
        print(r["path"], r.get("status", r.get("error")))
    print("wrote gitlab-spike-result.json")


if __name__ == "__main__":
    main()
