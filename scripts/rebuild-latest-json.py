#!/usr/bin/env python3
"""Rebuild a complete Tauri updater latest.json from a GitHub Release's assets.

Matrix builds each upload a partial latest.json; concurrent uploads can race and
drop platform keys (notably Windows). This script lists every signed updater
artifact on the release and writes a single authoritative latest.json.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path


def run(cmd: list[str]) -> str:
    return subprocess.check_output(cmd, text=True).strip()


def gh_json(args: list[str]):
    return json.loads(run(["gh", *args]))


def asset_platform_keys(name: str) -> list[str]:
    """Map a release asset filename to Tauri updater platform keys."""
    lower = name.lower()
    if lower.endswith(".sig"):
        return []

    if lower.endswith(".app.tar.gz"):
        if "aarch64" in lower:
            return ["darwin-aarch64", "darwin-aarch64-app"]
        if "x64" in lower or "x86_64" in lower:
            return ["darwin-x86_64", "darwin-x86_64-app"]
        return []

    if lower.endswith(".appimage"):
        return ["linux-x86_64", "linux-x86_64-appimage"]
    if lower.endswith(".deb"):
        return ["linux-x86_64-deb"]
    if lower.endswith(".rpm"):
        return ["linux-x86_64-rpm"]

    # Windows: NSIS setup maps to both the typed and generic keys when preferred.
    if lower.endswith("-setup.exe") or (lower.endswith(".exe") and "setup" in lower):
        return ["windows-x86_64-nsis", "windows-x86_64"]
    if lower.endswith(".msi"):
        return ["windows-x86_64-msi"]

    return []


def find_sig(assets: list[dict], artifact_name: str) -> dict | None:
    want = f"{artifact_name}.sig"
    for asset in assets:
        if asset["name"] == want:
            return asset
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, help="owner/name")
    parser.add_argument("--tag", required=True, help="release tag, e.g. v0.3.1")
    parser.add_argument(
        "--prefer-nsis",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Map generic windows-x86_64 to the NSIS setup.exe (default: true)",
    )
    args = parser.parse_args()

    release = gh_json(["api", f"repos/{args.repo}/releases/tags/{args.tag}"])
    assets = release.get("assets", [])
    version = args.tag.lstrip("v")
    notes = release.get("body") or ""
    if len(notes) > 4000:
        notes = notes[:4000]

    platforms: dict[str, dict[str, str]] = {}

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        for asset in assets:
            name = asset["name"]
            keys = asset_platform_keys(name)
            if not keys:
                continue
            if not find_sig(assets, name):
                print(f"skip {name}: missing .sig", file=sys.stderr)
                continue

            run(
                [
                    "gh",
                    "release",
                    "download",
                    args.tag,
                    "--repo",
                    args.repo,
                    "--pattern",
                    f"{name}.sig",
                    "-D",
                    str(tmp_path),
                    "--clobber",
                ]
            )
            sig_file = tmp_path / f"{name}.sig"
            if not sig_file.exists():
                print(f"skip {name}: could not download signature", file=sys.stderr)
                continue

            signature = sig_file.read_text().strip()
            entry = {"signature": signature, "url": asset["browser_download_url"]}

            for key in keys:
                if key == "windows-x86_64":
                    is_nsis = name.lower().endswith("-setup.exe") or "setup" in name.lower()
                    if args.prefer_nsis and not is_nsis and key in platforms:
                        continue
                    if not args.prefer_nsis and is_nsis and key in platforms:
                        continue
                platforms[key] = entry

    if not platforms:
        print("error: no updater platforms found on release", file=sys.stderr)
        return 1

    for key in ("windows-x86_64", "windows-x86_64-nsis"):
        if key not in platforms:
            print(f"warning: missing expected Windows key: {key}", file=sys.stderr)

    payload = {
        "version": version,
        "notes": notes,
        "pub_date": datetime.now(timezone.utc)
        .isoformat(timespec="milliseconds")
        .replace("+00:00", "Z"),
        "platforms": platforms,
    }

    out = Path("latest.json")
    out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {out} with platforms: {', '.join(sorted(platforms))}")

    run(
        [
            "gh",
            "release",
            "upload",
            args.tag,
            str(out),
            "--repo",
            args.repo,
            "--clobber",
        ]
    )
    print(f"uploaded latest.json to {args.tag}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
