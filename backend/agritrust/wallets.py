import hmac
from hashlib import sha256

from django.conf import settings
from django.core.cache import cache
from django.utils.crypto import get_random_string

CHALLENGE_TTL_SECONDS = 300


def make_challenge(wallet_address: str) -> str:
    nonce = get_random_string(32)
    challenge = f"AgriTrust wallet login:{wallet_address}:{nonce}"
    cache.set(_challenge_key(wallet_address), challenge, CHALLENGE_TTL_SECONDS)
    return challenge


def verify_signed_challenge(wallet_address: str, challenge: str, signature: str) -> bool:
    expected = cache.get(_challenge_key(wallet_address))
    if expected != challenge:
        return False

    verifier = getattr(settings, "WALLET_SIGNATURE_VERIFIER", "")
    if verifier:
        module_name, function_name = verifier.rsplit(".", 1)
        module = __import__(module_name, fromlist=[function_name])
        return bool(getattr(module, function_name)(wallet_address, challenge, signature))

    # Development fallback: deterministic HMAC so tests and local demos do not
    # need a Freighter browser session. Production should set WALLET_SIGNATURE_VERIFIER.
    expected_signature = hmac.new(
        settings.SECRET_KEY.encode(), f"{wallet_address}:{challenge}".encode(), sha256
    ).hexdigest()
    return hmac.compare_digest(signature, expected_signature)


def dev_signature(wallet_address: str, challenge: str) -> str:
    return hmac.new(
        settings.SECRET_KEY.encode(), f"{wallet_address}:{challenge}".encode(), sha256
    ).hexdigest()


def _challenge_key(wallet_address: str) -> str:
    return f"wallet-login:{wallet_address}"
