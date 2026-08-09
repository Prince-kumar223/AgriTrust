from django.db.models import Count, Sum
from drf_spectacular.utils import extend_schema
from rest_framework import permissions, viewsets
from rest_framework.authtoken.models import Token
from rest_framework.decorators import action, api_view, permission_classes, throttle_classes
from rest_framework.response import Response
from rest_framework.throttling import ScopedRateThrottle
from rest_framework.views import APIView

from .models import CropListing, Feedback, Offer, Trade, User
from .serializers import (
    AnalyticsSummarySerializer,
    CropListingSerializer,
    FeedbackSerializer,
    OfferSerializer,
    TradeSerializer,
    TxHashSerializer,
    UserSerializer,
    WalletChallengeSerializer,
    WalletLoginResponseSerializer,
    WalletLoginSerializer,
)


class WriteThrottleMixin:
    throttle_scope = "writes"


class WalletChallengeView(APIView):
    permission_classes = [permissions.AllowAny]
    throttle_classes = [ScopedRateThrottle]
    throttle_scope = "wallet_auth"

    @extend_schema(request=WalletChallengeSerializer, responses=WalletChallengeSerializer)
    def post(self, request):
        serializer = WalletChallengeSerializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        return Response(serializer.save())


class WalletLoginView(APIView):
    permission_classes = [permissions.AllowAny]
    throttle_classes = [ScopedRateThrottle]
    throttle_scope = "wallet_auth"

    @extend_schema(request=WalletLoginSerializer, responses=WalletLoginResponseSerializer)
    def post(self, request):
        serializer = WalletLoginSerializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        user = serializer.save()
        token, _ = Token.objects.get_or_create(user=user)
        return Response({"token": token.key, "user": UserSerializer(user).data})


class IsFarmer(permissions.BasePermission):
    def has_permission(self, request, view):
        return bool(
            request.user and request.user.is_authenticated and request.user.role == User.Role.FARMER
        )


class IsBuyer(permissions.BasePermission):
    def has_permission(self, request, view):
        return bool(
            request.user and request.user.is_authenticated and request.user.role == User.Role.BUYER
        )


class CropListingViewSet(WriteThrottleMixin, viewsets.ModelViewSet):
    queryset = CropListing.objects.select_related("farmer").all()
    serializer_class = CropListingSerializer

    def get_permissions(self):
        if self.action in {"create", "update", "partial_update", "destroy"}:
            return [IsFarmer()]
        return [permissions.IsAuthenticated()]

    def perform_create(self, serializer):
        serializer.save(farmer=self.request.user)

    def perform_destroy(self, instance):
        instance.status = CropListing.Status.CLOSED
        instance.save(update_fields=["status"])


class OfferViewSet(WriteThrottleMixin, viewsets.ModelViewSet):
    queryset = Offer.objects.select_related("listing", "buyer", "listing__farmer").all()
    serializer_class = OfferSerializer

    def get_permissions(self):
        if self.action == "create":
            return [IsBuyer()]
        return [permissions.IsAuthenticated()]

    def perform_create(self, serializer):
        offer = serializer.save(buyer=self.request.user)
        offer.listing.status = CropListing.Status.OFFERED
        offer.listing.save(update_fields=["status"])

    @action(detail=True, methods=["post"], permission_classes=[permissions.IsAuthenticated])
    def accept(self, request, pk=None):
        offer = self.get_object()
        if offer.listing.farmer != request.user:
            return Response(
                {"detail": "Only the listing farmer can accept this offer."}, status=403
            )
        offer.status = Offer.Status.ACCEPTED
        offer.save(update_fields=["status"])
        offer.listing.status = CropListing.Status.SOLD
        offer.listing.save(update_fields=["status"])
        return Response(self.get_serializer(offer).data)

    @action(detail=True, methods=["post"], permission_classes=[permissions.IsAuthenticated])
    def reject(self, request, pk=None):
        offer = self.get_object()
        if offer.listing.farmer != request.user:
            return Response(
                {"detail": "Only the listing farmer can reject this offer."}, status=403
            )
        offer.status = Offer.Status.REJECTED
        offer.save(update_fields=["status"])
        return Response(self.get_serializer(offer).data)


class TradeViewSet(WriteThrottleMixin, viewsets.ModelViewSet):
    queryset = Trade.objects.select_related("offer", "offer__listing", "offer__buyer").all()
    serializer_class = TradeSerializer

    def perform_create(self, serializer):
        trade = serializer.save()
        trade.offer.listing.status = CropListing.Status.SOLD
        trade.offer.listing.save(update_fields=["status"])

    @action(detail=True, methods=["post"])
    def record_deposit(self, request, pk=None):
        return self._record_transition(request, Trade.Status.FUNDED, "deposit_tx_hash")

    @action(detail=True, methods=["post"])
    def record_delivery(self, request, pk=None):
        return self._record_transition(request, Trade.Status.DELIVERED, "delivery_tx_hash")

    @action(detail=True, methods=["post"])
    def record_confirmation(self, request, pk=None):
        return self._record_transition(request, Trade.Status.COMPLETED, "confirmation_tx_hash")

    @action(detail=True, methods=["post"])
    def record_dispute(self, request, pk=None):
        return self._record_transition(request, Trade.Status.DISPUTED, "dispute_tx_hash")

    def _record_transition(self, request, next_status, tx_field):
        trade = self.get_object()
        serializer = TxHashSerializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        setattr(trade, tx_field, serializer.validated_data["tx_hash"])
        trade.status = next_status
        trade.save(update_fields=[tx_field, "status", "updated_at"])
        return Response(self.get_serializer(trade).data)


class FeedbackViewSet(WriteThrottleMixin, viewsets.ModelViewSet):
    queryset = Feedback.objects.select_related("user", "trade").all()
    serializer_class = FeedbackSerializer
    http_method_names = ["get", "post", "head", "options"]

    def perform_create(self, serializer):
        serializer.save(user=self.request.user)


@extend_schema(responses=AnalyticsSummarySerializer)
@api_view(["GET"])
@permission_classes([permissions.IsAdminUser])
@throttle_classes([])
def analytics_summary(request):
    total_trades = Trade.objects.count()
    completed_trades = Trade.objects.filter(status=Trade.Status.COMPLETED).count()
    total_volume = (
        Offer.objects.filter(trade__status=Trade.Status.COMPLETED).aggregate(
            total=Sum("offered_price")
        )["total"]
        or 0
    )
    by_status = dict(Trade.objects.values_list("status").annotate(count=Count("id")))
    completion_rate = completed_trades / total_trades if total_trades else 0
    return Response(
        {
            "total_trades": total_trades,
            "total_volume": total_volume,
            "completion_rate": completion_rate,
            "trades_by_status": by_status,
        }
    )
