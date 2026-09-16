"""Release helpers; run from the source checkout with Python 3.11+ and Cargo 1.90+."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tomllib
from urllib.error import HTTPError
from urllib.request import Request, urlopen


def output(*args):
    return subprocess.check_output(args, text=True).strip()


def release_target(prefer_existing_tag=False):
    with open("Cargo.toml", "rb") as manifest:
        version = tomllib.load(manifest)["workspace"]["package"]["version"]
    tag = os.environ.get("RELEASE_TAG") or f"v{version}"
    subprocess.run(["git", "check-ref-format", f"refs/tags/{tag}"], check=True)
    sha = output("git", "rev-parse", "HEAD")
    exists = subprocess.run(
        ["git", "show-ref", "--verify", "--quiet", f"refs/tags/{tag}"],
        check=False,
    )
    if exists.returncode not in (0, 1):
        exists.check_returncode()
    if exists.returncode == 0:
        tagged_sha = output("git", "rev-parse", f"refs/tags/{tag}^{{commit}}")
        if tagged_sha != sha:
            if not prefer_existing_tag:
                raise ValueError(
                    f"Tag {tag} points to {tagged_sha}, but source is {sha}. "
                    f"Select refs/tags/{tag} (or leave source_ref blank) to reuse the release; "
                    "bump the workspace version to release different code."
                )
            print(f"No source_ref selected; using existing tag {tag} at {tagged_sha}")
            sha = tagged_sha
            manifest = tomllib.loads(output("git", "show", f"{sha}:Cargo.toml"))
            version = manifest["workspace"]["package"]["version"]
    if tag != f"v{version}":
        raise ValueError(f"Release tag {tag!r} must match workspace version v{version}")
    return tag, sha, exists.returncode == 0


def index_versions(name):
    name = name.lower()
    if len(name) <= 2:
        prefix = str(len(name))
    elif len(name) == 3:
        prefix = f"3/{name[0]}"
    else:
        prefix = f"{name[:2]}/{name[2:4]}"
    request = Request(
        f"https://index.crates.io/{prefix}/{name}",
        headers={"User-Agent": "chromasync-release (github.com/spinualexandru/chromasync)"},
    )
    try:
        with urlopen(request, timeout=30) as response:
            return [json.loads(line) for line in response if line.strip()]
    except HTTPError as error:
        error.close()
        if error.code == 404:
            return []
        raise


def unpublished_packages(metadata):
    members = set(metadata["workspace_members"])
    selected = []
    for package in metadata["packages"]:
        registries = package["publish"]
        if package["id"] not in members or (
            registries is not None and "crates-io" not in registries
        ):
            continue
        name, version = package["name"], package["version"]
        published = next(
            (entry for entry in index_versions(name) if entry["vers"] == version), None
        )
        if published is not None:
            if published["yanked"]:
                raise ValueError(f"{name}@{version} is yanked; choose a new version")
            print(f"Skipping published {name}@{version}", flush=True)
        else:
            selected.append(f"{name}@{version}")
    return selected


def publish(dry_run):
    metadata = json.loads(output("cargo", "metadata", "--no-deps", "--format-version", "1"))
    selected = unpublished_packages(metadata)
    if not selected:
        print("All publishable workspace versions are already on crates.io.")
        return
    if not dry_run and not os.environ.get("CARGO_REGISTRY_TOKEN"):
        raise ValueError("Set the CARGO_REGISTRY_TOKEN repository secret before publishing")
    # Cargo handles dependency ordering, package verification, and index propagation.
    # Multiple -p flags also stage unpublished dependencies during --dry-run.
    command = ["cargo", "publish", "--registry", "crates-io"]
    for package in selected:
        command.extend(["--package", package])
    if dry_run:
        command.append("--dry-run")
    print("Running: " + " ".join(command), flush=True)
    subprocess.run(command, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["prepare", "tag", "publish"])
    parser.add_argument("--dry-run", action="store_true", help="Verify Cargo packages without uploading")
    args = parser.parse_args()
    if args.dry_run and args.command != "publish":
        parser.error("--dry-run is only supported with publish")
    if args.command == "publish":
        publish(args.dry_run)
        return
    tag, sha, exists = release_target(
        prefer_existing_tag=args.command == "prepare" and not os.environ.get("SOURCE_REF")
    )
    print(f"Release {tag} from {sha}; tag {'already exists' if exists else 'will be created'}")
    if args.command == "prepare":
        if path := os.environ.get("GITHUB_OUTPUT"):
            with Path(path).open("a") as stream:
                stream.write(f"tag={tag}\nsha={sha}\n")
    elif not exists:
        # Push the exact commit as a lightweight tag; never move an existing tag.
        subprocess.run(["git", "push", "origin", f"{sha}:refs/tags/{tag}"], check=True)


if __name__ == "__main__":
    main()
