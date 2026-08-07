"""
Test settings: identical to `config.settings` but run tests against an
in-memory SQLite database so CI does not require PostgreSQL.
"""

from .settings import *  # noqa: F401,F403  (intentional settings override)

DATABASES = {
    "default": {
        "ENGINE": "django.db.backends.sqlite3",
        "NAME": ":memory:",
    }
}
