"""Choose a release target from the latest stable GitHub version; default to local development."""
import argparse
from datetime import date
import json
from pathlib import Path
import re
import urllib.request
from versioning import core, development, write_version

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("kind", choices=["feature", "fix", "major"])
channel = parser.add_mutually_exclusive_group()
channel.add_argument("--stable", action="store_true", help="Prepare an intentional GitHub release without a dev suffix")
channel.add_argument("--beta", type=int, help="Prepare an intentional public beta with a positive sequence number")
parser.add_argument("--base", help="Verified numeric baseline: latest delivered target for fixes, latest published stable for features")
args = parser.parse_args()
if args.beta is not None and args.beta < 1:
    parser.error("Beta number must be positive")
if args.base:
    latest = args.base
else:
    request = urllib.request.Request("https://api.github.com/repos/OHmMieKunG/RoLauncher/releases/latest", headers={"User-Agent": "RoLauncher-release-preparation"})
    with urllib.request.urlopen(request, timeout=30) as response:
        latest = json.load(response)["tag_name"].removeprefix("v")
if core(latest) != latest:
    parser.error("The baseline must be a numeric MAJOR.MINOR.PATCH version")
major, minor, patch = map(int, latest.split("."))
parts = {"feature": (major, minor + 1, 0), "fix": (major, minor, patch + 1), "major": (major + 1, 0, 0)}[args.kind]
target = ".".join(map(str, parts))
version = target + f"-beta.{args.beta}" if args.beta is not None else target if args.stable else development(target)
write_version(root, version)
notes = root / f"docs/RELEASE-{target}.md"
if not notes.exists():
    notes.write_text(f"# RoLauncher {target}\n\nDescribe the user-visible changes, upgrade behavior and validation before opening the PR.\n", encoding="utf-8")
    changelog = root / "CHANGELOG.md"
    changelog.write_text(changelog.read_text(encoding="utf-8").replace("# Changelog\n", f"# Changelog\n\n## [{target}] — Unreleased\n\nDescribe the changes and validation before opening the PR.\n", 1), encoding="utf-8")
    desktop = root / "desktop/CHANGELOG.txt"
    desktop.write_text(f"{target} — Upcoming release\n\nDescribe the changes before opening the PR.\n\n" + desktop.read_text(encoding="utf-8"), encoding="utf-8")
print(f"Prepared {version} from {'verified base' if args.base else 'published stable'} {latest}. Update the consolidated {notes.name} notes; local iterations keep this target.")
