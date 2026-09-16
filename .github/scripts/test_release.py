import contextlib
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

import release


def package(name, publish=None, version="1.2.3"):
    return {"id": name, "name": name, "version": version, "publish": publish}


def metadata(*packages):
    return {"packages": list(packages), "workspace_members": [p["id"] for p in packages]}


class PublishTests(unittest.TestCase):
    @patch("release.index_versions", return_value=[])
    def test_respects_workspace_and_registry_publish_restrictions(self, versions):
        data = metadata(package("cli"), package("types", ["crates-io"]),
                        package("docs", []), package("internal", ["private"]))
        data["packages"].append(package("non-member"))
        self.assertEqual(release.unpublished_packages(data), ["cli@1.2.3", "types@1.2.3"])
        self.assertEqual(versions.call_count, 2)

    @patch("release.index_versions", return_value=[{"vers": "1.2.3", "yanked": False}])
    def test_partial_release_skips_only_exact_published_versions(self, _):
        self.assertEqual(
            release.unpublished_packages(metadata(package("types"), package("cli", version="1.2.4"))),
            ["cli@1.2.4"],
        )

    @patch("release.index_versions", return_value=[{"vers": "1.2.3", "yanked": True}])
    def test_yanked_version_fails(self, _):
        with self.assertRaisesRegex(ValueError, "yanked"):
            release.unpublished_packages(metadata(package("types")))

    @patch("release.urlopen")
    def test_only_registry_404_means_missing(self, open_url):
        for status in (403, 429, 500):
            open_url.side_effect = HTTPError("url", status, "failure", {}, None)
            with self.assertRaises(HTTPError):
                release.index_versions("types")
        open_url.side_effect = HTTPError("url", 404, "missing", {}, None)
        self.assertEqual(release.index_versions("types"), [])

    @patch("release.urlopen")
    def test_sparse_index_paths_and_json_lines(self, open_url):
        for name, path in [("a", "1/a"), ("ab", "2/ab"), ("Abc", "3/a/abc"),
                           ("chromasync-types", "ch/ro/chromasync-types")]:
            open_url.return_value.__enter__.return_value = io.BytesIO(
                b'{"vers":"1.2.3","yanked":false}\n'
            )
            self.assertEqual(release.index_versions(name), [{"vers": "1.2.3", "yanked": False}])
            self.assertEqual(open_url.call_args.args[0].full_url, f"https://index.crates.io/{path}")

    @patch("release.subprocess.run")
    @patch("release.unpublished_packages", return_value=["cli@1.2.3", "types@1.2.3"])
    @patch("release.output", return_value="{}")
    def test_passes_all_packages_to_cargo_for_dependency_ordering(self, _, selected, run):
        with patch.dict(os.environ, {}, clear=True):
            release.publish(dry_run=True)
        self.assertEqual(run.call_args.args[0], [
            "cargo", "publish", "--registry", "crates-io",
            "--package", "cli@1.2.3", "--package", "types@1.2.3", "--dry-run",
        ])
        with patch.dict(os.environ, {"CARGO_REGISTRY_TOKEN": "test"}):
            release.publish(dry_run=False)
        self.assertNotIn("--dry-run", run.call_args.args[0])
        run.side_effect = subprocess.CalledProcessError(101, "cargo publish")
        with self.assertRaises(subprocess.CalledProcessError):
            release.publish(dry_run=True)

    @patch("release.subprocess.run")
    @patch("release.unpublished_packages", return_value=[])
    @patch("release.output", return_value="{}")
    def test_complete_release_is_noop_without_token(self, _, selected, run):
        with patch.dict(os.environ, {}, clear=True):
            release.publish(dry_run=False)
        run.assert_not_called()

    @patch("release.subprocess.run")
    @patch("release.unpublished_packages", return_value=["cli@1.2.3"])
    @patch("release.output", return_value="{}")
    def test_missing_token_prevents_upload(self, _, selected, run):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "CARGO_REGISTRY_TOKEN"):
                release.publish(dry_run=False)
        run.assert_not_called()


class TagTests(unittest.TestCase):
    def setUp(self):
        self.temp = self.enterContext(tempfile.TemporaryDirectory())
        self.enterContext(contextlib.chdir(self.temp))
        self.enterContext(patch.dict(os.environ, {"RELEASE_TAG": ""}))
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "Release Test")
        Path("Cargo.toml").write_text('[workspace.package]\nversion = "1.2.3"\n')
        self.git("add", "Cargo.toml")
        self.git("commit", "-qm", "Initial")
        self.sha = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        return subprocess.check_output(["git", *args], text=True, stderr=subprocess.STDOUT)

    def test_missing_tag_is_derived_and_prepare_does_not_create_it(self):
        self.assertEqual(release.release_target(), ("v1.2.3", self.sha, False))
        self.assertEqual(self.git("tag", "--list"), "")

    def test_lightweight_and_annotated_tags_are_reused(self):
        for args in [("tag", "v1.2.3"), ("tag", "-a", "v1.2.3", "-m", "Release")]:
            self.git(*args)
            self.assertEqual(release.release_target(), ("v1.2.3", self.sha, True))
            self.git("tag", "-d", "v1.2.3")

    def test_existing_tag_on_another_commit_fails(self):
        self.git("tag", "v1.2.3")
        self.git("commit", "--allow-empty", "-qm", "New source")
        with self.assertRaisesRegex(ValueError, "points to"):
            release.release_target()

    def test_version_mismatch_fails(self):
        with patch.dict(os.environ, {"RELEASE_TAG": "v9.0.0"}):
            with self.assertRaisesRegex(ValueError, "must match"):
                release.release_target()

    def test_prepare_outputs_and_tag_creation_are_repeatable(self):
        remote = str(Path(self.temp) / "remote.git")
        self.git("init", "--bare", "-q", remote)
        self.git("remote", "add", "origin", remote)
        result = Path(self.temp) / "output"
        with patch("sys.argv", ["release.py", "prepare"]), patch.dict(
            os.environ, {"GITHUB_OUTPUT": str(result)}
        ):
            release.main()
        self.assertEqual(result.read_text(), f"tag=v1.2.3\nsha={self.sha}\n")
        with patch("sys.argv", ["release.py", "tag"]):
            release.main()
            self.git("fetch", "origin", "--tags")
            release.main()
        self.assertIn(self.sha, self.git("ls-remote", "--tags", "origin", "refs/tags/v1.2.3"))


if __name__ == "__main__":
    unittest.main()
