import base64
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import MagicMock, patch
import sys
import types


spec = importlib.util.spec_from_file_location("session_backend", Path(__file__).parents[1] / "backend.py")
backend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backend)
REAL_CREDENTIAL_BACKEND = backend.credential_backend
ACCESS = "ACCESS-TOKEN-DO-NOT-LEAK"
REFRESH = "REFRESH-TOKEN-DO-NOT-LEAK"


class MemoryVault:
    """Tests never open, read or modify the user's real OS credential store."""
    def __init__(self):
        self.items = {}
        self.fail_read = self.fail_write = self.fail_delete = False
        self.reads = 0

    def get_password(self, service, user):
        self.reads += 1
        if self.fail_read:
            raise RuntimeError(REFRESH)
        return self.items.get((service, user))

    def set_password(self, service, user, password):
        if self.fail_write:
            raise RuntimeError(REFRESH)
        self.items[service, user] = password

    def delete_password(self, service, user):
        if self.fail_delete:
            raise RuntimeError(REFRESH)
        self.items.pop((service, user), None)


class SessionTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.vault = MemoryVault()
        values = {"DATA": self.root, "GAME": self.root / "minecraft",
                  "CONFIG": self.root / "config.json", "profile": None, "pending": None,
                  "profile_client": "", "profile_deadline": 0., "session_refresh_token": ""}
        for name, value in values.items():
            mock = patch.object(backend, name, value)
            mock.start()
            self.addCleanup(mock.stop)
        vault = patch.object(backend, "credential_backend", return_value=self.vault)
        self.vault_factory = vault.start()
        self.addCleanup(vault.stop)
        emit = patch.object(backend, "emit")
        self.emit = emit.start()
        self.addCleanup(emit.stop)

    def response(self, refresh=REFRESH, access=ACCESS):
        return {"name": "Player", "id": "a" * 32, "access_token": access, "refresh_token": refresh}

    def seed(self):
        backend.accept_online_profile(self.response(), "client")
        backend.remember_session("client")

    def restart(self):
        backend.profile = backend.pending = None
        backend.profile_client = backend.session_refresh_token = ""
        backend.profile_deadline = 0.

    def assert_no_leaks(self):
        self.assertNotIn(ACCESS, str(self.emit.call_args_list))
        self.assertNotIn(REFRESH, str(self.emit.call_args_list))
        for path in self.root.rglob("*.json"):
            self.assertNotIn(ACCESS, path.read_text())
            self.assertNotIn(REFRESH, path.read_text())

    def test_login_persists_refresh_only_and_keeps_pkce(self):
        with patch.object(backend.mc.microsoft_account, "get_secure_login_data",
                          return_value=("https://login.example", "state", "verifier")):
            backend.dispatch({"action": "login", "client_id": "client"})
        with patch.object(backend.mc.microsoft_account, "parse_auth_code_url", return_value="code") as parse, \
             patch.object(backend.mc.microsoft_account, "complete_login", return_value=self.response()) as login:
            backend.dispatch({"action": "complete_login", "url": "private-return-url"})
        parse.assert_called_once_with("private-return-url", "state")
        login.assert_called_once_with("client", None, backend.REDIRECT, "code", "verifier")
        stored = json.loads(next(iter(self.vault.items.values())))
        self.assertEqual(stored, {"schema": 1, "client_id": "client", "refresh_token": REFRESH})
        self.assertTrue(backend.session_state()["restore"])
        self.assertEqual(json.loads(backend.CONFIG.read_text())["client_id"], "client")
        self.assert_no_leaks()

    def test_restart_restores_and_rotates_refresh_token(self):
        self.seed()
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh",
                          return_value=self.response(refresh="ROTATED")) as refresh:
            self.assertTrue(backend.restore_session("client"))
        refresh.assert_called_once_with("client", None, backend.REDIRECT, REFRESH)
        self.assertEqual(backend.profile["name"], "Player")
        self.assertEqual(json.loads(next(iter(self.vault.items.values())))["refresh_token"], "ROTATED")
        self.assertTrue(backend.session_state()["restore"])
        self.assert_no_leaks()

    def test_init_restores_microsoft_session_automatically(self):
        self.seed()
        backend.settings({"client_id": "client"})
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh", return_value=self.response()) as refresh, \
             patch.object(backend, "installed"), patch.object(backend.mc.utils, "get_version_list", return_value=[]):
            backend.dispatch({"action": "init"})
        refresh.assert_called_once()
        self.assertEqual(backend.profile["name"], "Player")

    def test_local_mode_does_not_access_vault_or_refresh_on_startup(self):
        self.seed()
        backend.settings({"client_id": "client", "offline": True})
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh") as refresh, \
             patch.object(backend, "installed"), patch.object(backend.mc.utils, "get_version_list", return_value=[]):
            backend.dispatch({"action": "init"})
        refresh.assert_not_called()
        self.assertEqual(self.vault.reads, 0)

    def test_transient_network_error_preserves_saved_credential(self):
        self.seed()
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh", side_effect=RuntimeError(REFRESH)):
            self.assertFalse(backend.restore_session("client"))
        self.assertIsNone(backend.profile)
        self.assertTrue(backend.session_state()["restore"])
        self.assertEqual(len(self.vault.items), 1)
        self.assert_no_leaks()

    def test_revoked_refresh_is_forgotten(self):
        self.seed()
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh",
                          side_effect=backend.mc.exceptions.InvalidRefreshToken()):
            self.assertFalse(backend.restore_session("client"))
        self.assertFalse(backend.session_state()["restore"])
        self.assertEqual(self.vault.items, {})
        self.assertIsNone(backend.profile)
        self.assert_no_leaks()

    def test_locked_vault_is_retryable_and_never_falls_back_to_file(self):
        self.seed()
        self.restart()
        self.vault.fail_read = True
        self.assertFalse(backend.restore_session("client"))
        self.assertTrue(backend.session_state()["restore"])
        self.vault.fail_read = False
        with patch.object(backend.mc.microsoft_account, "complete_refresh", return_value=self.response()):
            self.assertTrue(backend.restore_session("client"))
        self.assert_no_leaks()

    def test_failed_save_does_not_restore_previous_account(self):
        self.seed()
        self.vault.fail_write = True
        backend.accept_online_profile(self.response(refresh="NEW-ACCOUNT"), "new-client")
        backend.remember_session("new-client")
        self.assertEqual(backend.profile_client, "new-client")
        self.assertFalse(backend.session_state()["restore"])
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh") as refresh:
            self.assertFalse(backend.restore_session("client"))
        refresh.assert_not_called()
        self.assert_no_leaks()

    def test_logout_deletes_credential_and_prevents_restart_restore(self):
        self.seed()
        backend.dispatch({"action": "logout"})
        self.assertIsNone(backend.profile)
        self.assertEqual(backend.session_refresh_token, "")
        self.assertEqual(self.vault.items, {})
        self.assertFalse(backend.session_state()["restore"])
        self.assertFalse(backend.restore_session("client"))

    def test_logout_while_vault_locked_leaves_durable_tombstone(self):
        self.seed()
        self.vault.fail_read = True
        backend.forget_session()
        self.assertIsNone(backend.profile)
        self.assertFalse(backend.session_state()["restore"])
        self.assertTrue(backend.session_state()["cleanup_pending"])
        self.vault.fail_read = False
        self.restart()
        reads = self.vault.reads
        self.assertFalse(backend.restore_session("client"))
        self.assertEqual(self.vault.reads, reads)
        backend.forget_session()
        self.assertFalse(backend.session_state()["cleanup_pending"])
        self.assertEqual(self.vault.items, {})

    def test_logout_does_not_claim_success_if_tombstone_cannot_be_written(self):
        self.seed()
        with patch.object(backend, "write_json", side_effect=OSError("disk full")):
            with self.assertRaises(OSError):
                backend.forget_session()
        self.assertIsNotNone(backend.profile)

    def test_changed_client_id_never_refreshes_another_application(self):
        self.seed()
        self.restart()
        with patch.object(backend.mc.microsoft_account, "complete_refresh") as refresh:
            self.assertFalse(backend.restore_session("other-client"))
        refresh.assert_not_called()
        self.assertEqual(len(self.vault.items), 1)
        self.assert_no_leaks()

    def test_expired_access_token_is_refreshed_before_launch(self):
        self.seed()
        backend.profile_deadline = 0.
        with patch.object(backend.mc.microsoft_account, "complete_refresh",
                          return_value=self.response(access="FRESH")) as refresh:
            identity = backend.launch_identity({"offline": False, "client_id": "client"})
        refresh.assert_called_once()
        self.assertEqual(identity["access_token"], "FRESH")
        self.assertGreater(backend.profile_deadline, backend.time.monotonic())
        self.assert_no_leaks()

    def test_expiring_jwt_schedules_renewal_before_its_expiry(self):
        claim = base64.urlsafe_b64encode(json.dumps({"exp": 1100}).encode()).decode().rstrip("=")
        with patch.object(backend.time, "time", return_value=1000), \
             patch.object(backend.time, "monotonic", return_value=2000):
            backend.accept_online_profile(self.response(access=f"header.{claim}.signature"), "client")
        self.assertEqual(backend.profile_deadline, 2040)

    def test_corrupt_state_and_symlink_are_not_used_to_read_credentials(self):
        path = self.root / "session-state.json"
        path.write_text("{bad")
        self.assertFalse(backend.restore_session("client"))
        self.assertEqual(self.vault.reads, 0)
        path.unlink()
        other = self.root / "other.json"
        other.write_text('{"schema":1,"restore":true,"cleanup_pending":false}')
        path.symlink_to(other)
        self.assertFalse(backend.restore_session("client"))
        self.assertEqual(self.vault.reads, 0)

    def test_credential_namespace_isolated_by_data_directory(self):
        first = backend.credential_service()
        with patch.object(backend, "DATA", self.root / "another-launcher"):
            self.assertNotEqual(backend.credential_service(), first)

    def test_linux_uses_secret_service_not_configurable_plaintext_backend(self):
        keyring = types.ModuleType("keyring")
        keyring.get_keyring = MagicMock(side_effect=AssertionError("No automatic backend selection"))
        backends = types.ModuleType("keyring.backends")
        secret = types.ModuleType("keyring.backends.SecretService")
        secret.Keyring = MagicMock(return_value=MagicMock())
        modules = {"keyring": keyring, "keyring.backends": backends,
                   "keyring.backends.SecretService": secret}
        # This particular test exercises the real factory, not the fixture's memory vault.
        with patch.dict(sys.modules, modules), patch.object(sys, "platform", "linux"):
            vault = REAL_CREDENTIAL_BACKEND()
        secret.Keyring.assert_called_once_with()
        keyring.get_keyring.assert_not_called()
        self.assertIs(vault, secret.Keyring.return_value)
        self.assertEqual(vault.appid, "SkyLauncher")

    def test_microsoft_requests_have_timeouts(self):
        with patch.object(backend.requests.Session, "request", return_value=object()) as request:
            with backend.MicrosoftHTTP() as http:
                http.post("https://login.live.com/oauth20_token.srf", data={"test": "not-a-token"})
        self.assertEqual(request.call_args.kwargs["timeout"], (10, 20))

    def test_auth_http_adapter_is_scoped_and_restored_after_failure(self):
        original = backend.mc.microsoft_account.requests
        def failure():
            self.assertIsInstance(backend.mc.microsoft_account.requests, backend.MicrosoftHTTP)
            self.assertIs(backend.requests, original)
            raise RuntimeError("simulated transport failure")
        with self.assertRaises(RuntimeError):
            backend.microsoft_call(failure)
        self.assertIs(backend.mc.microsoft_account.requests, original)


if __name__ == "__main__":
    unittest.main()
