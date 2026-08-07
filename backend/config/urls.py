"""Root URL configuration for the AgriTrust backend."""

from django.contrib import admin
from django.urls import path

urlpatterns = [
    path("admin/", admin.site.urls),
    # API endpoints (see docs/api-spec.md) will be registered here in a later phase.
]
