import unittest

from lib.password import hash_password, is_bcrypt, verify_password


class PasswordCompatibilityTest(unittest.TestCase):
    def test_python_password_uses_bcrypt_and_verifies_shared_rust_shape(self):
        stored = hash_password("공유암호")
        self.assertTrue(stored.startswith("$2b$12$"))
        self.assertTrue(is_bcrypt(stored))
        self.assertTrue(verify_password(stored, "공유암호"))
        self.assertFalse(verify_password(stored, "틀린암호"))

    def test_legacy_plaintext_is_verifiable_only_for_one_time_upgrade(self):
        self.assertTrue(verify_password("옛암호", "옛암호"))
        self.assertFalse(verify_password("옛암호", "틀린암호"))
        upgraded = hash_password("옛암호")
        self.assertTrue(is_bcrypt(upgraded))
        self.assertNotEqual(upgraded, "옛암호")


if __name__ == "__main__":
    unittest.main()
