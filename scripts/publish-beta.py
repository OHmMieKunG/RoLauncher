"""Publish checked Windows artifacts as a prerelease, preserving stable/latest."""
import hashlib
from versioning import core
from release_files import checked_files


def publish(call, files, version, sha, notes):
    core(version)
    if "-dev." in version:
        raise ValueError("Development builds must never be published to GitHub; prepare an intentional stable/beta release first")
    tag = ("v" if "-beta." in version else "beta-v") + version
    existing_tag = call("/git/ref/tags/" + tag)
    obj = existing_tag["object"]
    if obj["type"] == "tag":
        obj = call("/git/tags/" + obj["sha"])["object"]
    if obj["type"] != "commit" or obj["sha"] != sha:
        raise ValueError("Beta tag must already exist and point to the checked commit")
    release = call("/releases/tags/" + tag, missing_ok=True)
    if release and not release["draft"]:
        if not release["prerelease"]:
            raise ValueError("Existing tag belongs to a stable release")
        print(f"{tag} is already published; existing assets are preserved.")
        return release["html_url"]
    marker = f"<!-- beta-commit:{sha} -->"
    body = notes + "\n\n" + marker + "\n"
    if release:
        if marker not in (release.get("body") or "") or not release["prerelease"]:
            raise ValueError("Cannot resume a draft from another commit or release channel")
        call("/releases/" + str(release["id"]), "PATCH", {"body": body})
    else:
        # The pushed tag is mandatory. Never let the API create/move a version tag.
        release = call("/releases", "POST", {
            "tag_name": tag, "name": f"RoLauncher {version} Beta — Windows",
            "body": body, "draft": True, "prerelease": True, "make_latest": "false",
        })
    upload = release["upload_url"].split("{")[0]
    if not upload.startswith("https://uploads.github.com/repos/OHmMieKunG/RoLauncher/releases/"):
        raise ValueError("Unexpected upload host")
    path = "/releases/" + str(release["id"])
    assets = {asset["name"]: asset for asset in call(path + "/assets")}
    if set(assets) - set(files):
        raise ValueError("Draft contains unexpected assets")
    for filename, source in sorted(files.items()):
        if filename in assets:
            call("/releases/assets/" + str(assets[filename]["id"]), "DELETE")
        data = source if isinstance(source, bytes) else source.read_bytes()
        call(upload + "?name=" + filename, "POST", binary=data)
    uploaded = call(path + "/assets")
    if {asset["name"] for asset in uploaded} != set(files):
        raise ValueError("Incomplete beta upload")
    for asset in uploaded:
        source = files[asset["name"]]
        data = source if isinstance(source, bytes) else source.read_bytes()
        if asset["size"] != len(data):
            raise ValueError("Incomplete beta asset")
        if asset.get("digest") and asset["digest"] != "sha256:" + hashlib.sha256(data).hexdigest():
            raise ValueError("Uploaded beta checksum mismatch")
    result = call(path, "PATCH", {"draft": False, "prerelease": True, "make_latest": "false"})
    if result["draft"] or not result["prerelease"]:
        raise ValueError("Published release is not a beta")
    return result["html_url"]


def main():
    # Preserve the tested legacy library seam; CLI authority uses the common gate.
    from release_promotion import main as promote_main
    promote_main("beta")


if __name__ == "__main__":
    main()
