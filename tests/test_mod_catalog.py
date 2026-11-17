import hashlib
import importlib.util
import io
import json
import struct
import zlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlparse
import zipfile


spec = importlib.util.spec_from_file_location("mod_backend", Path(__file__).parents[1] / "backend.py")
backend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backend)


class ModCatalogTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for key, value in {"DATA": self.root, "GAME": self.root / "minecraft",
                           "CONFIG": self.root / "config.json"}.items():
            self.patch(key, value)
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as jar:
            jar.writestr("fabric.mod.json", '{"id":"test"}')
        self.jar = data.getvalue()
        self.version = "fabric-test"
        path = backend.version_dir(self.version)
        path.mkdir(parents=True)
        (path / (self.version + ".json")).write_text(json.dumps({
            "inheritsFrom": "1.21.1", "libraries": [{"name": "net.fabricmc:fabric-loader:1"}]}))
        self.req = {"action": "mod_catalog_install", "project_id": "Root", "version": self.version}
        self.releases = {}
        self.add_release("Root")
        self.emit = self.patch("emit")
        self.fetch = self.patch("fetch_json", side_effect=self.api)
        self.download = self.patch("fetch_bytes", return_value=self.jar)

    def patch(self, name, *args, **kwargs):
        mock = patch.object(backend, name, *args, **kwargs)
        value = mock.start()
        self.addCleanup(mock.stop)
        return value

    def add_release(self, project, dependencies=None, **fields):
        value = {"id": project + "V1", "project_id": project, "game_versions": ["1.21.1"],
                 "loaders": ["fabric"], "version_type": "release", "date_published": "2026-01-01",
                 "dependencies": dependencies or [], "files": [{
                     "filename": project + ".jar", "primary": True,
                     "url": "https://cdn.modrinth.com/data/" + project + "/mod.jar",
                     "hashes": {"sha512": hashlib.sha512(self.jar).hexdigest()}}]}
        value.update(fields)
        self.releases[project] = value
        return value

    def dep(self, project=None, version=None, kind="required"):
        return {"project_id": project, "version_id": version, "dependency_type": kind}

    def api(self, url, host):
        self.assertEqual(host, "api.modrinth.com")
        path = urlparse(url).path.removeprefix("/v2/")
        if path.startswith("version/"):
            return next(v for v in self.releases.values() if v["id"] == path.split("/")[1])
        project = path.split("/")[1]
        if path.endswith("/version"):
            params = parse_qs(urlparse(url).query)
            self.assertEqual(json.loads(params["loaders"][0]), ["fabric"])
            self.assertEqual(json.loads(params["game_versions"][0]), ["1.21.1"])
            return [self.releases[project]]
        return {"id": project, "project_type": "mod"}

    def assert_empty(self):
        self.assertEqual(list(backend.mods_dir().iterdir()), [])

    def test_search_filters_and_pagination_contract(self):
        self.fetch.side_effect = None
        self.fetch.return_value = {"hits": [{"project_id": "Root", "project_type": "mod",
                                            "title": "Root mod", "description": "Description",
                                            "author": "Author", "downloads": 42}], "total_hits": 21}
        backend.dispatch({"action": "mod_catalog_search", "version": self.version,
                          "query": "performance & fast", "offset": 20})
        params = parse_qs(urlparse(self.fetch.call_args.args[0]).query)
        self.assertEqual(params["query"], ["performance & fast"])
        self.assertEqual(params["offset"], ["20"])
        self.assertEqual(params["limit"], ["20"])
        self.assertEqual(json.loads(params["facets"][0]),
                         [["project_type:mod"], ["versions:1.21.1"], ["categories:fabric"]])
        self.emit.assert_any_call("mod_catalog", items=[{
            "project_id": "Root", "title": "Root mod", "description": "Description",
            "author": "Author", "downloads": 42}], minecraft_version="1.21.1", loader="fabric",
            offset=20, total_hits=21)
        self.download.assert_not_called()

    def test_search_validation_and_vanilla(self):
        for fields in ({"offset": -1}, {"offset": True}, {"offset": 1001},
                       {"query": "a" * 201}, {"query": []}, {"version": "1.21.1"}):
            with self.subTest(fields=fields), self.assertRaises((backend.UserError, ValueError)):
                backend.mod_catalog_search({"version": self.version, **fields})
        self.fetch.assert_not_called()

    def test_install_single_mod_and_events(self):
        backend.dispatch(self.req)
        self.assertEqual((backend.mods_dir() / "Root.jar").read_bytes(), self.jar)
        self.assertEqual([p.name for p in backend.mods_dir().iterdir()], ["Root.jar"])
        self.download.assert_called_once_with(
            "https://cdn.modrinth.com/data/Root/mod.jar", 64 * 1024 * 1024, {"cdn.modrinth.com"})
        self.assertEqual(self.emit.call_args_list[-2].args[0], "mods")
        self.assertEqual(self.emit.call_args_list[-1].args[0], "status")

    def test_recursive_required_pinned_and_shared_dependencies(self):
        self.add_release("Root", [self.dep("A"), self.dep("B"), self.dep("Ignore", kind="optional")])
        self.add_release("A", [self.dep(version="SharedV1")])
        self.add_release("B", [self.dep("Shared"), self.dep("Ignore", kind="embedded")])
        self.add_release("Shared")
        backend.dispatch(self.req)
        self.assertEqual({p.name for p in backend.mods_dir().iterdir()},
                         {"Root.jar", "A.jar", "B.jar", "Shared.jar"})
        self.assertEqual(self.download.call_count, 4)

    def test_install_uses_request_profile_not_global(self):
        p = backend.save_profile({"offline_name": "Alex", "version": self.version})
        backend.dispatch({**self.req, "offline": True, "local_profile": p["id"]})
        self.assertTrue((backend.profile_game(p) / "mods" / "Root.jar").is_file())
        self.assertFalse((backend.GAME / "mods" / "Root.jar").exists())

    def test_incompatibility_rejected_before_download(self):
        for fields in ({"game_versions": ["1.20"]}, {"loaders": ["forge"]},
                       {"version_type": "snapshot"}):
            self.add_release("Root", **fields)
            with self.subTest(fields=fields), self.assertRaises(backend.UserError):
                backend.dispatch(self.req)
        self.download.assert_not_called()
        self.assert_empty()

    def test_beta_release_used_when_no_stable_exists(self):
        self.add_release("Root", version_type="beta")
        backend.dispatch(self.req)
        self.assertTrue((backend.mods_dir() / "Root.jar").is_file())
        self.download.assert_called_once()

    def test_beta_pinned_dependency_accepted(self):
        self.add_release("Root", [self.dep("Dep", "DepV1")])
        self.add_release("Dep", id="DepV1", version_type="beta")
        backend.dispatch(self.req)
        self.assertTrue((backend.mods_dir() / "Dep.jar").is_file())

    def test_pinned_dependency_incompatible_or_wrong_project(self):
        self.add_release("Root", [self.dep("Dep", "DepV1")])
        self.add_release("Dep", loaders=["forge"])
        with self.assertRaises(backend.UserError):
            backend.dispatch(self.req)
        self.add_release("Dep")
        self.releases["Root"]["dependencies"] = [self.dep("Other", "DepV1")]
        with self.assertRaises(backend.UserError):
            backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_cycle_missing_dependency_and_depth_bound(self):
        self.add_release("Root", [self.dep("A")])
        self.add_release("A", [self.dep("Root")])
        with self.assertRaisesRegex(backend.UserError, "ciclo"):
            backend.dispatch(self.req)
        self.add_release("Root", [self.dep()])
        with self.assertRaises(backend.UserError):
            backend.dispatch(self.req)
        self.add_release("Root", [self.dep("P0")])
        for i in range(17):
            self.add_release("P" + str(i), [self.dep("P" + str(i + 1))])
        with self.assertRaisesRegex(backend.UserError, "niveles"):
            backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_project_count_bound(self):
        self.add_release("Root", [self.dep("P" + str(i)) for i in range(32)])
        for i in range(32):
            self.add_release("P" + str(i))
        with self.assertRaisesRegex(backend.UserError, "32 proyectos"):
            backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_conflicting_pinned_versions_rejected(self):
        self.add_release("Root", [self.dep("Dep"), self.dep("Dep", "DepV2")])
        self.add_release("Dep")
        other = self.add_release("Other", id="DepV2", project_id="Dep")
        self.assertEqual(other["project_id"], "Dep")
        with self.assertRaisesRegex(backend.UserError, "versiones distintas"):
            backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_latest_stable_release_selected_not_api_order(self):
        old = self.releases["Root"]
        new = {**old, "id": "RootV2", "date_published": "2026-11-01"}
        beta = {**old, "id": "RootV3", "date_published": "2026-11-02", "version_type": "beta"}
        original = self.api

        def api(url, host):
            return [old, beta, new] if "/Root/version?" in url else original(url, host)

        self.fetch.side_effect = api
        result = backend.mod_catalog_plan("Root", "1.21.1", "fabric")
        self.assertEqual([v["id"] for v in result], ["RootV2"])

    def test_network_failure_discards_staging(self):
        self.add_release("Root", [self.dep("Dep")])
        self.add_release("Dep")
        self.download.side_effect = [self.jar, backend.requests.Timeout()]
        with self.assertRaises(backend.requests.Timeout):
            backend.dispatch(self.req)
        self.assert_empty()

    def test_hash_failure_leaves_no_partial_install_or_staging(self):
        self.add_release("Root", [self.dep("Dep")])
        self.add_release("Dep")
        self.releases["Root"]["files"][0]["hashes"]["sha512"] = "0" * 128
        with self.assertRaisesRegex(backend.UserError, "hash SHA-512"):
            backend.dispatch(self.req)
        self.assertEqual(self.download.call_count, 2)
        self.assert_empty()

    def test_hash_required_and_jar_validated(self):
        self.releases["Root"]["files"][0]["hashes"] = {}
        with self.assertRaisesRegex(backend.UserError, "SHA-512"):
            backend.dispatch(self.req)
        self.download.assert_not_called()
        self.add_release("Root")["files"][0]["hashes"]["sha512"] = hashlib.sha512(b"bad").hexdigest()
        self.download.return_value = b"bad"
        with self.assertRaisesRegex(backend.UserError, "JAR"):
            backend.dispatch(self.req)
        self.assert_empty()

    def test_path_traversal_and_unsafe_filenames(self):
        for name in ("../bad.jar", "..\\bad.jar", "/bad.jar", ".hidden.jar",
                     "bad.jar.disabled", "bad\x00.jar", "a" * 200 + ".jar"):
            self.releases["Root"]["files"][0]["filename"] = name
            with self.subTest(name=name), self.assertRaises(backend.UserError):
                backend.dispatch(self.req)
        self.download.assert_not_called()
        self.assert_empty()

    def test_disallowed_urls_rejected_before_download(self):
        for url in ("http://cdn.modrinth.com/a.jar", "https://evil.com/a.jar",
                    "https://cdn.modrinth.com.evil.com/a.jar",
                    "https://user@cdn.modrinth.com/a.jar", "https://cdn.modrinth.com:99/a.jar"):
            self.releases["Root"]["files"][0]["url"] = url
            with self.subTest(url=url), self.assertRaises(backend.UserError):
                backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_existing_enabled_disabled_case_and_symlink_conflicts(self):
        for name in ("Root.jar", "Root.jar.disabled", "root.JAR"):
            path = backend.mods_dir() / name
            path.write_bytes(b"existing")
            with self.subTest(name=name), self.assertRaises(backend.UserError):
                backend.dispatch(self.req)
            self.assertEqual(path.read_bytes(), b"existing")
            path.unlink()
        path = backend.mods_dir() / "Root.jar"
        path.symlink_to(self.root / "absent")
        with self.assertRaises(backend.UserError):
            backend.dispatch(self.req)
        self.assertTrue(path.is_symlink())
        self.download.assert_not_called()

    def test_second_mod_reuses_identical_active_dependency_without_rewriting(self):
        self.add_release("Root", [self.dep("Shared")])
        self.add_release("Shared")
        backend.dispatch(self.req)
        shared = backend.mods_dir() / "Shared.jar"
        before = shared.stat()
        self.download.reset_mock()
        self.add_release("Second", [self.dep("Shared")])
        backend.dispatch({**self.req, "project_id": "Second"})
        self.download.assert_called_once_with(
            "https://cdn.modrinth.com/data/Second/mod.jar", 64 * 1024 * 1024, {"cdn.modrinth.com"})
        self.assertEqual(shared.stat().st_ino, before.st_ino)
        self.assertEqual(shared.stat().st_mtime_ns, before.st_mtime_ns)
        self.assertEqual(shared.read_bytes(), self.jar)

    def test_identical_disabled_dependency_is_never_reactivated(self):
        self.add_release("Root", [self.dep("Shared")])
        self.add_release("Shared")
        disabled = backend.mods_dir() / "Shared.jar.disabled"
        disabled.write_bytes(self.jar)
        with self.assertRaisesRegex(backend.UserError, "desactivado"):
            backend.dispatch(self.req)
        self.assertEqual(list(backend.mods_dir().iterdir()), [disabled])
        self.assertEqual(disabled.read_bytes(), self.jar)
        self.download.assert_not_called()

    def test_reused_dependency_disappearing_aborts_before_publish(self):
        self.add_release("Root", [self.dep("Shared")])
        self.add_release("Shared")
        shared = backend.mods_dir() / "Shared.jar"
        shared.write_bytes(self.jar)

        def download(*args):
            shared.unlink()
            return self.jar

        self.download.side_effect = download
        with self.assertRaisesRegex(backend.UserError, "cambió"):
            backend.dispatch(self.req)
        self.assert_empty()

    def make_skin(self, name="skin.png", color=b"\x40\x80\xc0\xff"):
        def chunk(kind, value):
            return (struct.pack(">I", len(value)) + kind + value
                    + struct.pack(">I", zlib.crc32(kind + value) & 0xffffffff))
        data = b"\x89PNG\r\n\x1a\n"
        data += chunk(b"IHDR", struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0))
        data += chunk(b"IDAT", zlib.compress((b"\0" + color * 64) * 64))
        data += chunk(b"IEND", b"")
        path = self.root / name
        path.write_bytes(data)
        return path

    def test_failed_skin_activation_preserves_profile_and_settings_byte_for_byte(self):
        old = backend.save_profile({"offline_name": "Alex", "version": "1.21.1",
                                    "skin_path": str(self.make_skin())})
        backend.settings({"offline": True, "local_profile": old["id"], "version": "1.21.1"})
        before = (self.root / "profiles.json").read_bytes(), backend.CONFIG.read_bytes()
        req = {"offline": True, "local_profile": old["id"], "offline_name": "Steve",
               "version": self.version, "skin_path": str(self.make_skin("new.png", b"\xff\0\0\xff")),
               "variant": "slim"}
        for failure in ("vanilla", "unavailable", "network", "hash"):
            with self.subTest(failure=failure):
                self.fetch.side_effect = backend.requests.Timeout() if failure == "network" else None
                self.fetch.return_value = [] if failure == "unavailable" else [self.releases["Root"]]
                self.download.return_value = b"tampered" if failure == "hash" else self.jar
                attempt = {**req, "version": "1.21.1"} if failure == "vanilla" else req
                with self.assertRaises((backend.UserError, backend.requests.Timeout)):
                    backend.enable_local_skin(attempt)
                self.assertEqual((self.root / "profiles.json").read_bytes(), before[0])
                self.assertEqual(backend.CONFIG.read_bytes(), before[1])
                self.assertEqual(backend.find_profile(old["id"]), old)
                self.assertEqual(backend.support_marker(old), {})
                self.assertFalse((backend.profile_game(old) / "mods" / "Root.jar").exists())

    def test_failed_skin_runtime_preparation_rolls_back_existing_activation(self):
        req = {"offline": True, "offline_name": "Alex", "version": self.version,
               "skin_path": str(self.make_skin())}
        self.fetch.side_effect = None
        self.fetch.return_value = [self.releases["Root"]]
        backend.enable_local_skin(req)
        old = backend.profile_store()["profiles"][0]
        req["local_profile"] = old["id"]
        game = backend.profile_game(old)
        watched = [self.root / "profiles.json", backend.CONFIG,
                   game / "CustomSkinLoader" / "CustomSkinLoader.json",
                   game / "CustomSkinLoader" / "SkyLauncher" / "skins" / "Alex.png",
                   game / "skylauncher-skin-support.json", game / "mods" / "Root.jar"]
        before = {p: p.read_bytes() for p in watched}
        req["skin_path"] = str(self.make_skin("new.png", b"\xff\0\0\xff"))
        original = backend.prepare_local_skin

        def prepare(p, game):
            original(p, game)
            raise OSError("simulated write failure")

        with patch.object(backend, "prepare_local_skin", side_effect=prepare):
            with self.assertRaises(OSError):
                backend.enable_local_skin(req)
        self.assertEqual({p: p.read_bytes() for p in watched}, before)
        self.assertEqual(backend.find_profile(old["id"]), old)
        self.assertFalse(any(p.name.startswith(".skin-activation-") for p in game.iterdir()))

    def test_failed_new_skin_activation_does_not_publish_new_profile(self):
        self.fetch.side_effect = None
        self.fetch.return_value = []
        with self.assertRaises(backend.UserError):
            backend.enable_local_skin({"offline": True, "offline_name": "Alex",
                                       "version": self.version, "skin_path": str(self.make_skin())})
        self.assertEqual(backend.profile_store()["profiles"], [])
        self.assertFalse(backend.CONFIG.exists())

    def test_saved_skin_without_activation_can_launch_vanilla(self):
        p = backend.save_profile({"offline_name": "Alex", "version": "1.21.1",
                                 "skin_path": str(self.make_skin())})
        with patch.object(backend.mc.install, "install_minecraft_version"), \
             patch.object(backend, "installed"), \
             patch.object(backend.mc.command, "get_minecraft_command", return_value=["java"]), \
             patch.object(backend.subprocess, "Popen") as popen:
            popen.return_value.wait.return_value = 0
            backend.dispatch({"action": "play", "offline": True, "offline_name": "Alex",
                              "local_profile": p["id"], "version": "1.21.1"})
        popen.assert_called_once()
        self.assertTrue(backend.find_profile(p["id"])["skin"])
        self.assertEqual(backend.support_marker(p), {})
        self.assertTrue(any("sin soporte activado" in c.kwargs.get("text", "")
                            for c in self.emit.call_args_list))

    def test_duplicate_filenames_fail_without_download(self):
        self.add_release("Root", [self.dep("Dep")])
        self.add_release("Dep")["files"][0]["filename"] = "root.jar"
        with self.assertRaisesRegex(backend.UserError, "mismo nombre"):
            backend.dispatch(self.req)
        self.download.assert_not_called()

    def test_commit_failure_rolls_back_only_our_files(self):
        self.add_release("Root", [self.dep("Dep")])
        self.add_release("Dep")
        original = backend.os.link

        def publish(source, destination):
            if destination.name == "Root.jar":
                destination.write_bytes(b"concurrent writer")
                raise FileExistsError()
            original(source, destination)

        with patch.object(backend.os, "link", side_effect=publish), self.assertRaises(FileExistsError):
            backend.dispatch(self.req)
        self.assertEqual({p.name for p in backend.mods_dir().iterdir()}, {"Root.jar"})
        self.assertEqual((backend.mods_dir() / "Root.jar").read_bytes(), b"concurrent writer")

    def test_non_mod_project_rejected(self):
        original = self.api

        def api(url, host):
            value = original(url, host)
            if url.endswith("/project/Root"):
                value["project_type"] = "modpack"
            return value

        self.fetch.side_effect = api
        with self.assertRaises(backend.UserError):
            backend.dispatch(self.req)
        self.download.assert_not_called()


if __name__ == "__main__":
    unittest.main()
