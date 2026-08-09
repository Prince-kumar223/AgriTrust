from django.contrib import admin
from django.contrib.auth.admin import UserAdmin

from .models import CropListing, Feedback, Offer, Trade, User


@admin.register(User)
class AgriTrustUserAdmin(UserAdmin):
    fieldsets = UserAdmin.fieldsets + (
        ("AgriTrust profile", {"fields": ("role", "wallet_address", "phone", "location")}),
    )
    add_fieldsets = UserAdmin.add_fieldsets + (
        ("AgriTrust profile", {"fields": ("role", "wallet_address", "phone", "location")}),
    )
    list_display = ("username", "wallet_address", "role", "location", "is_staff")
    search_fields = ("username", "wallet_address", "phone", "location")


@admin.register(CropListing)
class CropListingAdmin(admin.ModelAdmin):
    list_display = (
        "crop_type",
        "farmer",
        "quantity",
        "unit",
        "price_per_unit",
        "status",
        "created_at",
    )
    list_filter = ("status", "unit", "created_at")
    search_fields = ("crop_type", "farmer__wallet_address", "description")


@admin.register(Offer)
class OfferAdmin(admin.ModelAdmin):
    list_display = ("listing", "buyer", "offered_price", "status", "created_at")
    list_filter = ("status", "created_at")
    search_fields = ("listing__crop_type", "buyer__wallet_address", "message")


@admin.register(Trade)
class TradeAdmin(admin.ModelAdmin):
    list_display = ("on_chain_trade_id", "offer", "contract_address", "status", "updated_at")
    list_filter = ("status", "created_at", "updated_at")
    search_fields = ("on_chain_trade_id", "contract_address", "offer__listing__crop_type")


@admin.register(Feedback)
class FeedbackAdmin(admin.ModelAdmin):
    list_display = ("trade", "user", "rating", "created_at")
    list_filter = ("rating", "created_at")
    search_fields = ("trade__on_chain_trade_id", "user__wallet_address", "comment")


# Register your models here.
