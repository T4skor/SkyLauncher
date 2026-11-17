import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile
import hashlib
import io
import struct
import zlib
from unittest.mock import patch


spec = importlib.util.spec_from_file_location("backend", Path(__file__).parents[1] / "backend.py")
backend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backend)


class BackendTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name, value in {"DATA": self.root, "GAME": self.root / "minecraft", "CONFIG": self.root / "config.json", "profile": None, "pending": None,
                            "profile_client": "", "profile_deadline": float("inf"), "session_refresh_token": ""}.items():
            mock = patch.object(backend, name, value)
            mock.start()
            self.addCleanup(mock.stop)

    def test_engine_reads_utf8_json_with_windows_code_page(self):
        request = {
            "action": "save", "version": "1.21.1", "java": "C:\\Usuarios\\José\\java.exe",
            "ram": 4, "offline": True, "offline_name": "Alex",
        }
        result = subprocess.run(
            [sys.executable, "-u", str(Path(backend.__file__))],
            input=(json.dumps(request, ensure_ascii=False) + "\n").encode("utf-8"),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={**os.environ, "MI_LAUNCHER_DATA": str(self.root),
                 "PYTHONIOENCODING": "cp1252"},
            timeout=20,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
        events = [json.loads(line) for line in result.stdout.decode("utf-8").splitlines()]
        self.assertTrue(any(event["event"] == "done" for event in events), events)
        self.assertFalse(any(event["event"] == "error" for event in events), events)
        self.assertEqual(json.loads((self.root / "rust-config.json").read_text())["java"],
                         request["java"])

    def test_version_paths_cannot_escape_or_follow_symlinks(self):
        for version in ("", "..", "../worlds", "/tmp/worlds", "..\\worlds"):
            with self.assertRaises(ValueError):
                backend.version_dir(version)
        versions = backend.GAME / "versions"
        versions.mkdir(parents=True)
        outside = self.root / "outside"
        outside.mkdir()
        (versions / "linked").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            backend.version_dir("linked")

    def test_delete_preserves_worlds_and_other_versions(self):
        target = backend.version_dir("1.21")
        other = backend.version_dir("1.20")
        worlds = backend.GAME / "saves" / "My World"
        for directory in (target, other, worlds):
            directory.mkdir(parents=True)
            (directory / "keep.txt").write_text("data")
        with patch.object(backend, "installed"), patch.object(backend, "emit"):
            backend.dispatch({"action": "delete", "version": "1.21"})
        self.assertFalse(target.exists())
        self.assertTrue((other / "keep.txt").exists())
        self.assertTrue((worlds / "keep.txt").exists())

    def test_settings_exclude_credentials(self):
        backend.settings(dict(client_id="public", java="", version="1.21", ram=4, snapshots=False, access_token="secret"))
        saved = json.loads(backend.CONFIG.read_text())
        self.assertNotIn("access_token", saved)
        self.assertEqual(saved["ram"], 4)

    def test_offline_identity_matches_java_and_does_not_use_microsoft(self):
        backend.profile = {"name": "Online", "id": "online-id", "access_token": "secret"}
        identity = backend.launch_identity({"offline": True, "offline_name": " Notch "})
        self.assertEqual(identity, {"name": "Notch", "id": "b50ad385829d3141a2167e7d7539ba7f", "access_token": "0"})
        self.assertEqual(backend.profile["access_token"], "secret")
        self.assertNotEqual(identity["id"], backend.launch_identity({"offline": True, "offline_name": "Steve"})["id"])

    def test_offline_name_validation(self):
        for name in ("", "ab", "a" * 17, "a b", "../name", "José", "na\nme"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                backend.launch_identity({"offline": True, "offline_name": name})

    def test_microsoft_mode_still_requires_session(self):
        with self.assertRaises(ValueError):
            backend.launch_identity({"offline": False, "offline_name": "Steve"})

    def test_local_settings_persist_without_session(self):
        backend.settings(dict(client_id="", java="", version="1.21", ram=4, snapshots=False,
                              offline=True, offline_name=" Steve "))
        saved = json.loads(backend.CONFIG.read_text())
        self.assertTrue(saved["offline"])
        self.assertEqual(saved["offline_name"], "Steve")

    def make_mod(self, name="example.jar"):
        source = self.root / name
        with zipfile.ZipFile(source, "w") as archive:
            archive.writestr("fabric.mod.json", '{"id":"example","version":"1.0"}')
        return source

    def test_mod_import_toggle_delete_preserves_original(self):
        source = self.make_mod()
        original = source.read_bytes()
        with patch.object(backend, "emit") as emit:
            backend.dispatch({"action": "mod_import", "path": str(source)})
            active = backend.mod_path("example.jar")
            self.assertEqual(active.read_bytes(), original)
            backend.dispatch({"action": "mod_toggle", "name": "example.jar"})
            disabled = backend.mod_path("example.jar.disabled")
            self.assertFalse(active.exists())
            self.assertEqual(disabled.read_bytes(), original)
            events = [call.kwargs["mods"] for call in emit.call_args_list if call.args[0] == "mods"]
            self.assertFalse(events[-1][0]["enabled"])
            backend.dispatch({"action": "mod_toggle", "name": disabled.name})
            self.assertTrue(active.exists())
            backend.dispatch({"action": "mod_delete", "name": active.name})
            self.assertFalse(active.exists())
        self.assertEqual(source.read_bytes(), original)

    def test_mod_import_rejects_invalid_and_duplicate_archives(self):
        source = self.make_mod()
        with patch.object(backend, "emit"):
            backend.dispatch({"action": "mod_import", "path": str(source)})
            backend.dispatch({"action": "mod_toggle", "name": source.name})
            with self.assertRaises(backend.UserError):
                backend.dispatch({"action": "mod_import", "path": str(source)})
            invalid = self.root / "invalid.jar"
            invalid.write_text("not a jar")
            with self.assertRaises(backend.UserError):
                backend.dispatch({"action": "mod_import", "path": str(invalid)})
            self.assertFalse((backend.mods_dir() / invalid.name).exists())

    def test_mod_paths_and_collision_protection(self):
        for name in ("../outside.jar", "..\\outside.jar", "config.json", "/tmp/mod.jar"):
            with self.assertRaises(backend.UserError):
                backend.mod_path(name)
        source = self.make_mod()
        (backend.mods_dir() / "linked.jar").symlink_to(source)
        with self.assertRaises(backend.UserError):
            backend.dispatch({"action": "mod_delete", "name": "linked.jar"})
        active = backend.mod_path("example.jar")
        active.write_bytes(b"first")
        disabled = backend.mod_path("example.jar.disabled")
        disabled.write_bytes(b"second")
        with self.assertRaises(backend.UserError):
            backend.dispatch({"action": "mod_toggle", "name": active.name})
        self.assertEqual(active.read_bytes(), b"first")
        self.assertEqual(disabled.read_bytes(), b"second")

    def test_fabric_install_selects_and_persists_result(self):
        with patch.object(backend.mc.mod_loader, "get_mod_loader") as get_loader, \
             patch.object(backend, "installed"), patch.object(backend, "emit") as emit:
            get_loader.return_value.install.return_value = "fabric-loader-test-1.21"
            backend.dispatch(dict(action="fabric_install", client_id="", java="/custom/java",
                                  version="1.21", ram=4, snapshots=False))
            get_loader.assert_called_once_with("fabric")
            self.assertEqual(get_loader.return_value.install.call_args.kwargs["java"], "/custom/java")
            emit.assert_any_call("selected_version", version="fabric-loader-test-1.21")
            self.assertEqual(json.loads(backend.CONFIG.read_text())["version"], "fabric-loader-test-1.21")

    def test_login_passes_validated_state_and_pkce_without_exposing_token(self):
        backend.pending = ("client", "state", "verifier")
        with patch.object(backend.mc.microsoft_account, "parse_auth_code_url", return_value="code") as parse, \
             patch.object(backend.mc.microsoft_account, "complete_login", return_value={"name": "Player", "id": "a" * 32, "access_token": "secret"}) as login, \
             patch.object(backend, "emit") as emit:
            backend.dispatch({"action": "complete_login", "url": "redirect"})
            parse.assert_called_once_with("redirect", "state")
            login.assert_called_once_with("client", None, backend.REDIRECT, "code", "verifier")
            self.assertNotIn("secret", str(emit.call_args_list))
        self.assertIsNone(backend.pending)

    def make_skin(self, height=64):
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)
        data = b"\x89PNG\r\n\x1a\n"
        data += chunk(b"IHDR", struct.pack(">IIBBBBB", 64, height, 8, 6, 0, 0, 0))
        data += chunk(b"IDAT", zlib.compress((b"\0" + b"\x40\x80\xc0\xff" * 64) * height))
        data += chunk(b"IEND", b"")
        path = self.root / f"skin-{height}.png"
        path.write_bytes(data)
        return path

    def profile_request(self, **fields):
        value = dict(client_id="", java="", version="1.21.1", ram=4, snapshots=False,
                     offline=True, offline_name="Alex")
        value.update(fields)
        return value

    def make_profile(self, skin=False, **fields):
        req = self.profile_request(skin_path=str(self.make_skin()) if skin else "", **fields)
        with patch.object(backend, "emit"):
            p = backend.save_profile(req)
        return p

    def make_loader_metadata(self, loader="fabric"):
        name = loader + "-test-1.21.1"
        path = backend.version_dir(name)
        path.mkdir(parents=True)
        prefix = {"fabric": "net.fabricmc:fabric-loader", "forge": "net.minecraftforge:forge",
                  "neoforge": "net.neoforged:neoforge", "quilt": "org.quiltmc:quilt-loader"}[loader]
        (path / (name + ".json")).write_text(json.dumps({
            "inheritsFrom": "1.21.1", "libraries": [{"name": prefix + ":1.0"}]}))
        return name

    def jar_bytes(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as archive:
            archive.writestr("test.txt", "official artifact stand-in")
        return data.getvalue()

    def mock_support(self, jar):
        return [{"game_versions": ["1.21.1"], "loaders": list(backend.LOADERS),
                 "version_type": "release", "dependencies": [], "files": [{
                     "primary": True, "filename": "CustomSkinLoader_test.jar",
                     "url": "https://cdn.modrinth.com/data/idMHQ4n2/test.jar",
                     "hashes": {"sha512": hashlib.sha512(jar).hexdigest()}}]}]

    def test_profiles_persist_skin_copy_and_keep_instances_separate(self):
        p = self.make_profile(skin=True)
        q = self.make_profile()
        self.assertNotEqual(p["id"], q["id"])
        original = self.make_skin()
        data = original.read_bytes()
        original.unlink()
        self.assertEqual(backend.skin_path(p).read_bytes(), data)
        self.assertEqual(backend.find_profile(p["id"])["version"], "1.21.1")
        self.assertNotEqual(backend.profile_game(p), backend.profile_game(q))
        self.assertNotEqual(backend.profile_game(p), backend.GAME)
        self.assertTrue((self.root / "profiles.json.bak").exists())

    def test_profile_update_keeps_id_and_does_not_persist_credentials(self):
        p = self.make_profile()
        req = self.profile_request(local_profile=p["id"], skin_path="", access_token="secret", refresh_token="secret")
        req["offline_name"] = "Steve"
        req["version"] = "1.20.1"
        with patch.object(backend, "emit"):
            q = backend.save_profile(req)
        self.assertEqual(q["id"], p["id"])
        self.assertEqual(q["name"], "Steve")
        self.assertEqual(len(backend.profile_store()["profiles"]), 1)
        self.assertNotIn("secret", (self.root / "profiles.json").read_text())

    def test_corrupt_profiles_are_never_overwritten(self):
        self.root.mkdir(exist_ok=True)
        path = self.root / "profiles.json"
        path.write_text("{broken")
        with self.assertRaises(backend.UserError):
            self.make_profile()
        self.assertEqual(path.read_text(), "{broken")

    def test_profile_paths_and_managed_directory_symlinks_rejected(self):
        with self.assertRaises(backend.UserError):
            backend.find_profile("../outside")
        outside = self.root / "outside"
        outside.mkdir()
        (self.root / "skins").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(backend.UserError):
            self.make_profile(skin=True)
        self.assertEqual(list(outside.iterdir()), [])

    def test_png_validation_rejects_corruption_wrong_size_and_legacy_slim(self):
        path = self.make_skin()
        data = path.read_bytes()
        backend.validate_skin(data)
        for invalid in (b"not a png", data[:-8], data[:40] + b"x" + data[41:], b"x" * (backend.MAX_SKIN + 1)):
            with self.subTest(size=len(invalid)), self.assertRaises(backend.UserError):
                backend.validate_skin(invalid)
        with patch.object(backend, "emit"), self.assertRaises(backend.UserError):
            backend.save_profile(self.profile_request(skin_path=str(self.make_skin(32)), variant="slim"))
        self.assertEqual(backend.profile_store()["profiles"], [])

    def test_published_texture_url_accepts_only_minecraft_textures(self):
        url = "https://textures.minecraft.net/texture/" + "a" * 64
        self.assertEqual(
            backend.published_texture_url({"data": {"texture": {"url": url}}}), url)
        self.assertEqual(
            backend.published_texture_url({"data": {"texture": {"urls": {"skin": url}}}}), url)
        for value in (None, {}, {"data": "bad"}, {"data": {"texture": {"url": "http://textures.minecraft.net/texture/" + "a" * 64}}},
                      {"data": {"texture": {"url": "https://example.com/skin.png"}}},
                      {"data": {"texture": {"url": "https://textures.minecraft.net/texture/XYZ"}}}):
            with self.subTest(value=value):
                self.assertIsNone(backend.published_texture_url(value))

    def test_publish_skin_sends_png_and_emits_texture_url(self):
        path = self.make_skin()
        url = "https://textures.minecraft.net/texture/" + "b" * 64

        class Response:
            status_code = 200

            def __enter__(self):
                return self

            def __exit__(self, *args):
                return False

            def iter_content(self, size):
                yield json.dumps({"data": {"texture": {"url": url}}}).encode()

        with patch.object(backend.requests, "post", return_value=Response()) as post, \
             patch.object(backend, "emit") as emit:
            backend.publish_skin({"skin_path": str(path), "variant": "slim"})
        self.assertEqual(post.call_args.kwargs["data"], {"variant": "slim", "visibility": "0"})
        self.assertEqual(post.call_args.kwargs["files"]["file"][1], path.read_bytes())
        emit.assert_any_call("skin_published", url=url, variant="slim")

    def test_publish_skin_reports_rate_limit_and_missing_texture(self):
        path = self.make_skin()

        class Response:
            def __init__(self, status, body=b""):
                self.status_code, self.body = status, body

            def __enter__(self):
                return self

            def __exit__(self, *args):
                return False

            def iter_content(self, size):
                yield self.body

        with patch.object(backend.requests, "post", return_value=Response(429)), \
             patch.object(backend, "emit"):
            with self.assertRaises(backend.UserError):
                backend.publish_skin({"skin_path": str(path), "variant": "classic"})
        with patch.object(backend.requests, "post", return_value=Response(200, b'{"data":{}}')), \
             patch.object(backend, "emit") as emit:
            with self.assertRaises(backend.UserError):
                backend.publish_skin({"skin_path": str(path), "variant": "classic"})
        emit.assert_not_called()

    def test_profile_selection_restores_name_version_and_skin(self):
        p = self.make_profile(skin=True)
        with patch.object(backend, "emit") as emit:
            backend.dispatch({"action": "profile_select", "id": p["id"]})
        cfg = json.loads(backend.CONFIG.read_text())
        self.assertEqual(cfg["local_profile"], p["id"])
        self.assertEqual(cfg["offline_name"], "Alex")
        self.assertTrue(cfg["offline"])
        events = [call.kwargs["value"] for call in emit.call_args_list if call.args[0] == "local_profile_selected"]
        self.assertEqual(events[0]["skin_path"], str(backend.skin_path(p)))

    def test_profile_delete_preserves_worlds_and_skin(self):
        p = self.make_profile(skin=True)
        game = backend.profile_game(p)
        world = game / "saves" / "World"
        world.mkdir(parents=True)
        (world / "level.dat").write_text("world")
        with patch.object(backend, "emit"):
            backend.dispatch({"action": "profile_delete", "id": p["id"]})
        self.assertEqual(backend.profile_store()["profiles"], [])
        self.assertTrue((world / "level.dat").exists())
        self.assertTrue(backend.skin_path(p).exists())

    def test_legacy_local_player_migrates_without_moving_worlds(self):
        backend.settings(self.profile_request())
        with patch.object(backend.mc.utils, "get_version_list", return_value=[]), \
             patch.object(backend, "installed"), patch.object(backend, "emit"):
            backend.dispatch({"action": "init"})
            backend.dispatch({"action": "init"})
        profiles = backend.profile_store()["profiles"]
        self.assertEqual(len(profiles), 1)
        self.assertIs(profiles[0]["legacy"], True)
        self.assertEqual(backend.profile_game(profiles[0]), backend.GAME)

    def test_mod_libraries_are_scoped_to_profile(self):
        p = self.make_profile()
        q = self.make_profile()
        source = self.make_mod()
        context = {"offline": True, "local_profile": p["id"]}
        with patch.object(backend, "emit"):
            backend.dispatch({"action": "mod_import", "path": str(source), **context})
        self.assertTrue(backend.mod_path(source.name, context).exists())
        self.assertFalse(backend.mod_path(source.name, {"offline": True, "local_profile": q["id"]}).exists())
        self.assertFalse(backend.mod_path(source.name).exists())

    def test_all_loaders_install_and_update_selected_profile(self):
        for loader in backend.LOADERS:
            with self.subTest(loader=loader):
                p = self.make_profile()
                req = self.profile_request(action="loader_install", loader=loader, local_profile=p["id"])
                with patch.object(backend.mc.mod_loader, "get_mod_loader") as get_loader, \
                     patch.object(backend, "installed"), patch.object(backend, "emit"):
                    get_loader.return_value.install.return_value = loader + "-installed"
                    backend.dispatch(req)
                    get_loader.assert_called_once_with(loader)
                    self.assertEqual(get_loader.return_value.install.call_args.args[0], "1.21.1")
                self.assertEqual(backend.find_profile(p["id"])["version"], loader + "-installed")

    def test_loader_switch_resolves_base_and_rejects_unsupported(self):
        version = self.make_loader_metadata("fabric")
        req = self.profile_request(action="loader_install", loader="forge")
        req["version"] = version
        with patch.object(backend.mc.mod_loader, "get_mod_loader") as get_loader, \
             patch.object(backend, "emit"):
            get_loader.return_value.install.side_effect = backend.mc.exceptions.UnsupportedVersion("1.21.1")
            with self.assertRaisesRegex(backend.UserError, "Forge no admite"):
                backend.dispatch(req)
            self.assertEqual(get_loader.return_value.install.call_args.args[0], "1.21.1")
        req["loader"] = "untrusted"
        with self.assertRaises(backend.UserError):
            backend.dispatch(req)

    def test_cannot_uninstall_a_version_referenced_by_a_profile(self):
        self.make_profile()
        with self.assertRaisesRegex(backend.UserError, "perfil local"):
            backend.dispatch({"action": "delete", "version": "1.21.1"})

    def test_local_launch_uses_instance_but_shared_version_resources(self):
        p = self.make_profile()
        req = self.profile_request(action="play", local_profile=p["id"])
        with patch.object(backend, "emit"), patch.object(backend, "installed"), \
             patch.object(backend.mc.install, "install_minecraft_version") as install, \
             patch.object(backend.mc.command, "get_minecraft_command", return_value=["java"]) as command, \
             patch.object(backend.subprocess, "Popen") as popen:
            popen.return_value.wait.return_value = 0
            backend.dispatch(req)
            self.assertEqual(install.call_args.args[1], str(backend.GAME))
            options = command.call_args.args[2]
            self.assertEqual(options["gameDirectory"], str(backend.profile_game(p)))
            self.assertEqual(options["token"], "0")
            self.assertEqual(popen.call_args.kwargs["cwd"], backend.profile_game(p))

    def test_skin_support_installs_verified_mod_and_prioritizes_local_skin(self):
        version = self.make_loader_metadata("neoforge")
        p = self.make_profile(skin=True)
        req = self.profile_request(action="local_skin_enable", local_profile=p["id"],
                                   skin_path=str(backend.skin_path(p)), variant="slim")
        req["version"] = version
        jar = self.jar_bytes()
        with patch.object(backend, "emit"), patch.object(backend, "fetch_json", return_value=self.mock_support(jar)), \
             patch.object(backend, "fetch_bytes", return_value=jar):
            backend.dispatch(req)
        p = backend.find_profile(p["id"])
        marker = backend.support_marker(p)
        self.assertEqual(marker["version"], version)
        self.assertEqual(marker["sha512"], hashlib.sha512(jar).hexdigest())
        root = backend.profile_game(p) / "CustomSkinLoader"
        cfg = json.loads((root / "CustomSkinLoader.json").read_text())
        self.assertEqual(cfg["loadlist"][0]["name"], "SkyLauncherLocal")
        self.assertEqual(cfg["loadlist"][0]["model"], "slim")
        self.assertTrue((root / "SkyLauncher/skins/Alex.png").is_file())
        self.assertEqual(cfg["loadlist"][1]["type"], "MojangAPI")

    def test_mod_hash_mismatch_is_not_installed(self):
        version = self.make_loader_metadata()
        p = self.make_profile(skin=True)
        req = self.profile_request(local_profile=p["id"], skin_path=str(backend.skin_path(p)))
        req["version"] = version
        with patch.object(backend, "emit"), patch.object(backend, "fetch_json", return_value=self.mock_support(self.jar_bytes())), \
             patch.object(backend, "fetch_bytes", return_value=b"tampered"):
            with self.assertRaisesRegex(backend.UserError, "hash"):
                backend.enable_local_skin(req)
        self.assertFalse((backend.profile_game(p) / "mods/CustomSkinLoader_test.jar").exists())
        self.assertEqual(backend.support_marker(p), {})

    def test_skin_launch_blocked_without_matching_support_and_removal_cleans_config(self):
        p = self.make_profile(skin=True)
        backend.prepare_local_skin(p, backend.profile_game(p))
        req = self.profile_request(action="play", local_profile=p["id"])
        with patch.object(backend, "emit"), patch.object(backend.mc.install, "install_minecraft_version") as install:
            with self.assertRaisesRegex(backend.UserError, "Guardar y activar"):
                backend.dispatch(req)
            install.assert_not_called()
        req.update(skin_path="", action="profile_save")
        with patch.object(backend, "emit"):
            backend.dispatch(req)
        cfg = json.loads((backend.profile_game(p) / "CustomSkinLoader/CustomSkinLoader.json").read_text())
        self.assertFalse(any(e["name"] == "SkyLauncherLocal" for e in cfg["loadlist"]))
        self.assertEqual(backend.find_profile(p["id"])["skin"], "")

    def test_skin_support_requires_real_loader_and_compatible_release(self):
        p = self.make_profile(skin=True)
        req = self.profile_request(local_profile=p["id"], skin_path=str(backend.skin_path(p)))
        with patch.object(backend, "emit"), self.assertRaisesRegex(backend.UserError, "instala"):
            backend.enable_local_skin(req)
        req["version"] = self.make_loader_metadata()
        with patch.object(backend, "emit"), patch.object(backend, "fetch_json", return_value=[]):
            with self.assertRaisesRegex(backend.UserError, "compatible"):
                backend.enable_local_skin(req)

    def test_catalog_is_public_and_selection_restores_actual_model(self):
        path = self.make_skin()
        skin_id, texture = "a" * 32, "b" * 64
        data = {"success": True, "skins": [{"uuid": skin_id, "name": "Example", "texture": texture}],
                "pagination": {"next": {"after": skin_id}}}
        with patch.object(backend, "fetch_json", return_value=data), \
             patch.object(backend, "texture_skin", return_value=path), patch.object(backend, "emit") as emit:
            backend.catalog({})
            emit.assert_any_call("skin_catalog", skins=[{"id": skin_id, "name": "Example", "path": str(path)}], after=skin_id)
        data = {"success": True, "skin": {"visibility": "public", "variant": "slim",
                                        "texture": {"hash": {"skin": texture}}}}
        with patch.object(backend, "fetch_json", return_value=data), \
             patch.object(backend, "texture_skin", return_value=path), patch.object(backend, "emit") as emit:
            backend.choose_catalog_skin({"id": skin_id})
            emit.assert_any_call("skin", path=str(path), variant="slim")
        with self.assertRaises(backend.UserError):
            backend.catalog({"after": "../private"})
        with self.assertRaises(backend.UserError):
            backend.choose_catalog_skin({"id": "not-an-id"})

    def test_remote_download_only_uses_allowlisted_https_without_redirects(self):
        for url in ("http://cdn.modrinth.com/mod.jar", "https://evil.example/mod.jar",
                    "https://cdn.modrinth.com@evil.example/mod.jar", "file:///etc/passwd"):
            with self.subTest(url=url), self.assertRaises(backend.UserError):
                backend.fetch_bytes(url, 1024, {"cdn.modrinth.com"})
        with patch.object(backend.requests, "get") as get:
            response = get.return_value.__enter__.return_value
            response.status_code = 302
            with self.assertRaises(backend.UserError):
                backend.fetch_bytes("https://cdn.modrinth.com/mod.jar", 1024, {"cdn.modrinth.com"})
            self.assertFalse(get.call_args.kwargs["allow_redirects"])
            response.status_code = 200
            response.iter_content.return_value = [b"a" * 1025]
            with self.assertRaises(backend.UserError):
                backend.fetch_bytes("https://cdn.modrinth.com/mod.jar", 1024, {"cdn.modrinth.com"})

    def test_disabling_skin_mod_updates_profile_readiness_and_blocks_launch(self):
        p = self.make_profile(skin=True)
        req = self.profile_request(action="local_skin_enable", local_profile=p["id"],
                                   version=self.make_loader_metadata(), skin_path=str(backend.skin_path(p)))
        jar = self.jar_bytes()
        with patch.object(backend, "emit"), patch.object(backend, "fetch_json", return_value=self.mock_support(jar)), \
             patch.object(backend, "fetch_bytes", return_value=jar):
            backend.dispatch(req)
        with patch.object(backend, "emit") as emit:
            backend.dispatch({**req, "action": "mod_toggle", "name": "CustomSkinLoader_test.jar"})
            profiles = [c.kwargs["profiles"] for c in emit.call_args_list if c.args[0] == "local_profiles"][-1]
            self.assertEqual(profiles[0]["skin_support_version"], "")
        with patch.object(backend, "emit"), patch.object(backend.mc.install, "install_minecraft_version") as install:
            with self.assertRaises(backend.UserError):
                backend.dispatch({**req, "action": "play"})
            install.assert_not_called()

    def test_altered_skin_mod_never_executes(self):
        p = self.make_profile(skin=True)
        req = self.profile_request(action="local_skin_enable", local_profile=p["id"],
                                   version=self.make_loader_metadata(), skin_path=str(backend.skin_path(p)))
        jar = self.jar_bytes()
        with patch.object(backend, "emit"), patch.object(backend, "fetch_json", return_value=self.mock_support(jar)), \
             patch.object(backend, "fetch_bytes", return_value=jar):
            backend.dispatch(req)
        mod = backend.mod_path("CustomSkinLoader_test.jar", req)
        mod.write_bytes(b"changed after installation")
        with patch.object(backend, "emit"), patch.object(backend.subprocess, "Popen") as popen:
            with self.assertRaisesRegex(backend.UserError, "ha cambiado"):
                backend.dispatch({**req, "action": "play"})
            popen.assert_not_called()


if __name__ == "__main__":
    unittest.main()
