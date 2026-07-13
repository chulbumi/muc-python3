# -*- coding: utf-8 -*-

"""Password hashing shared by the legacy Python login path.

The Rust server stores bcrypt (`$2b$`) hashes.  Python's stdlib `crypt`
uses the same system bcrypt implementation, so the comparison server can
read and write the shared character files without restoring plaintext.
"""

import crypt
import hmac


def is_bcrypt(value):
    return isinstance(value, str) and value.startswith(('$2a$', '$2b$', '$2y$'))


def hash_password(plain):
    salt = crypt.mksalt(crypt.METHOD_BLOWFISH, rounds=2 ** 12)
    return crypt.crypt(str(plain), salt)


def verify_password(stored, plain):
    stored = str(stored)
    plain = str(plain)
    if is_bcrypt(stored):
        candidate = crypt.crypt(plain, stored)
        return candidate is not None and hmac.compare_digest(candidate, stored)
    # One-time compatibility with old character files. Successful callers
    # immediately replace this value with bcrypt.
    return hmac.compare_digest(stored.encode('utf-8'), plain.encode('utf-8'))
