from decimal import Decimal

from django.contrib.auth import get_user_model
from rest_framework import serializers

from .models import CropListing, Feedback, Offer, Trade
from .wallets import make_challenge, verify_signed_challenge

User = get_user_model()


class UserSerializer(serializers.ModelSerializer):
    class Meta:
        model = User
        fields = ["id", "username", "role", "wallet_address", "phone", "location"]
        read_only_fields = ["id"]


class WalletChallengeSerializer(serializers.Serializer):
    wallet_address = serializers.CharField(max_length=128)

    def create(self, validated_data):
        return {"challenge": make_challenge(validated_data["wallet_address"])}


class WalletLoginSerializer(serializers.Serializer):
    wallet_address = serializers.CharField(max_length=128)
    signature = serializers.CharField()
    challenge = serializers.CharField()
    role = serializers.ChoiceField(
        choices=User.Role.choices, required=False, default=User.Role.BUYER
    )
    username = serializers.CharField(required=False, allow_blank=True)
    phone = serializers.CharField(required=False, allow_blank=True)
    location = serializers.CharField(required=False, allow_blank=True)

    def validate(self, attrs):
        if not verify_signed_challenge(
            attrs["wallet_address"],
            attrs["challenge"],
            attrs["signature"],
        ):
            raise serializers.ValidationError("Invalid wallet signature or expired challenge.")
        return attrs

    def create(self, validated_data):
        wallet_address = validated_data["wallet_address"]
        defaults = {
            "username": validated_data.get("username") or wallet_address[-12:],
            "role": validated_data.get("role", User.Role.BUYER),
            "phone": validated_data.get("phone", ""),
            "location": validated_data.get("location", ""),
        }
        user, created = User.objects.get_or_create(wallet_address=wallet_address, defaults=defaults)
        if not created:
            for field in ("role", "phone", "location"):
                if field in validated_data:
                    setattr(user, field, validated_data[field])
            user.save(update_fields=["role", "phone", "location"])
        return user


class WalletLoginResponseSerializer(serializers.Serializer):
    token = serializers.CharField()
    user = UserSerializer()


class CropListingSerializer(serializers.ModelSerializer):
    farmer = UserSerializer(read_only=True)

    class Meta:
        model = CropListing
        fields = [
            "id",
            "farmer",
            "crop_type",
            "quantity",
            "unit",
            "price_per_unit",
            "description",
            "status",
            "created_at",
        ]
        read_only_fields = ["id", "farmer", "status", "created_at"]

    def validate_quantity(self, value):
        if value <= Decimal("0"):
            raise serializers.ValidationError("Quantity must be greater than zero.")
        return value

    def validate_price_per_unit(self, value):
        if value < Decimal("0"):
            raise serializers.ValidationError("Price per unit cannot be negative.")
        return value


class OfferSerializer(serializers.ModelSerializer):
    buyer = UserSerializer(read_only=True)

    class Meta:
        model = Offer
        fields = ["id", "listing", "buyer", "offered_price", "message", "status", "created_at"]
        read_only_fields = ["id", "buyer", "status", "created_at"]

    def validate_offered_price(self, value):
        if value < Decimal("0"):
            raise serializers.ValidationError("Offer price cannot be negative.")
        return value

    def validate_listing(self, value):
        if value.status not in {CropListing.Status.ACTIVE, CropListing.Status.OFFERED}:
            raise serializers.ValidationError("Offers can only be made on open listings.")
        return value


class TradeSerializer(serializers.ModelSerializer):
    class Meta:
        model = Trade
        fields = [
            "id",
            "offer",
            "on_chain_trade_id",
            "contract_address",
            "status",
            "create_tx_hash",
            "deposit_tx_hash",
            "delivery_tx_hash",
            "confirmation_tx_hash",
            "dispute_tx_hash",
            "created_at",
            "updated_at",
        ]
        read_only_fields = [
            "id",
            "status",
            "deposit_tx_hash",
            "delivery_tx_hash",
            "confirmation_tx_hash",
            "dispute_tx_hash",
            "created_at",
            "updated_at",
        ]

    def validate_offer(self, value):
        if value.status != Offer.Status.ACCEPTED:
            raise serializers.ValidationError("Trade records require an accepted offer.")
        return value


class TxHashSerializer(serializers.Serializer):
    tx_hash = serializers.CharField(max_length=128)


class AnalyticsSummarySerializer(serializers.Serializer):
    total_trades = serializers.IntegerField()
    total_volume = serializers.DecimalField(max_digits=14, decimal_places=2)
    completion_rate = serializers.FloatField()
    trades_by_status = serializers.DictField(child=serializers.IntegerField())


class FeedbackSerializer(serializers.ModelSerializer):
    user = UserSerializer(read_only=True)

    class Meta:
        model = Feedback
        fields = ["id", "user", "trade", "rating", "comment", "created_at"]
        read_only_fields = ["id", "user", "created_at"]

    def validate(self, attrs):
        if attrs["trade"].status != Trade.Status.COMPLETED:
            raise serializers.ValidationError("Feedback can only be posted for completed trades.")
        return attrs
