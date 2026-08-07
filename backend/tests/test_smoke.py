"""Scaffold smoke tests: verify the Django project boots and imports cleanly."""


def test_project_settings_load():
    from config.settings import BASE_DIR, INSTALLED_APPS

    assert BASE_DIR.exists()
    assert "rest_framework" in INSTALLED_APPS
    assert "corsheaders" in INSTALLED_APPS


def test_root_urlconf_resolves():
    from config.urls import urlpatterns

    assert urlpatterns
