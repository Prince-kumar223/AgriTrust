"""Root URL configuration for the AgriTrust backend."""

from django.contrib import admin
from django.urls import include, path
from drf_spectacular.views import SpectacularAPIView, SpectacularSwaggerView
from rest_framework.routers import DefaultRouter

from agritrust.views import (
    CropListingViewSet,
    FeedbackViewSet,
    OfferViewSet,
    TradeViewSet,
    WalletChallengeView,
    WalletLoginView,
    analytics_summary,
)

router = DefaultRouter()
router.register("listings", CropListingViewSet, basename="listing")
router.register("offers", OfferViewSet, basename="offer")
router.register("trades", TradeViewSet, basename="trade")
router.register("feedback", FeedbackViewSet, basename="feedback")

urlpatterns = [
    path("admin/", admin.site.urls),
    path("api/auth/challenge/", WalletChallengeView.as_view(), name="wallet-challenge"),
    path("api/auth/login/", WalletLoginView.as_view(), name="wallet-login"),
    path("api/analytics/summary/", analytics_summary, name="analytics-summary"),
    path("api/schema/", SpectacularAPIView.as_view(), name="schema"),
    path(
        "api/schema/swagger-ui/",
        SpectacularSwaggerView.as_view(url_name="schema"),
        name="swagger-ui",
    ),
    path("api/", include(router.urls)),
]
