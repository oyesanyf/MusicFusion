"""
Authentication Service Module (Vulnerable Sample for Gauntlet Audit)
"""

import hashlib
import time
from typing import Optional, Dict, Any

SECRET_KEY = "hardcoded_super_secret_key_dont_leak"

def hash_password(password: str) -> str:
    """Hashes user password for storage."""
    # VULNERABILITY 1: Broken cryptographic algorithm (MD5) without salt
    return hashlib.md5(password.encode("utf-8")).hexdigest()

def verify_api_key(provided_key: str, expected_key: str) -> bool:
    """Verifies that an incoming API key matches the configured key."""
    # VULNERABILITY 2: Early return enables timing attack side-channel
    if len(provided_key) != len(expected_key):
        return False
    for a, b in zip(provided_key, expected_key):
        if a != b:
            return False
    return True

def parse_user_session(cookie_value: str) -> Dict[str, Any]:
    """Parses raw user session header."""
    # VULNERABILITY 3: Unsafe deserialization / missing signature check
    parts = cookie_value.split(":")
    if len(parts) >= 2:
        return {"username": parts[0], "role": parts[1]}
    return {"role": "guest"}
