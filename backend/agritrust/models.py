from django.conf import settings
from django.contrib.auth.models import AbstractUser
from django.core.validators import MaxValueValidator, MinValueValidator
from django.db import models


class User(AbstractUser):
    class Role(models.TextChoices):
        FARMER = "farmer", "Farmer"
        BUYER = "buyer", "Buyer"

    role = models.CharField(max_length=12, choices=Role.choices, default=Role.BUYER)
    wallet_address = models.CharField(max_length=128, unique=True)
    phone = models.CharField(max_length=32, blank=True)
    location = models.CharField(max_length=255, blank=True)

    def __str__(self) -> str:
        return f"{self.username} ({self.role})"


class CropListing(models.Model):
    class Status(models.TextChoices):
        ACTIVE = "active", "Active"
        OFFERED = "offered", "Offered"
        SOLD = "sold", "Sold"
        CLOSED = "closed", "Closed"

    farmer = models.ForeignKey(
        settings.AUTH_USER_MODEL, on_delete=models.CASCADE, related_name="crop_listings"
    )
    crop_type = models.CharField(max_length=120)
    quantity = models.DecimalField(max_digits=12, decimal_places=2)
    unit = models.CharField(max_length=32)
    price_per_unit = models.DecimalField(max_digits=12, decimal_places=2)
    description = models.TextField(blank=True)
    status = models.CharField(max_length=12, choices=Status.choices, default=Status.ACTIVE)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        ordering = ["-created_at"]

    def __str__(self) -> str:
        return f"{self.crop_type} by {self.farmer}"


class Offer(models.Model):
    class Status(models.TextChoices):
        PENDING = "pending", "Pending"
        ACCEPTED = "accepted", "Accepted"
        REJECTED = "rejected", "Rejected"

    listing = models.ForeignKey(CropListing, on_delete=models.CASCADE, related_name="offers")
    buyer = models.ForeignKey(
        settings.AUTH_USER_MODEL, on_delete=models.CASCADE, related_name="offers"
    )
    offered_price = models.DecimalField(max_digits=12, decimal_places=2)
    message = models.TextField(blank=True)
    status = models.CharField(max_length=12, choices=Status.choices, default=Status.PENDING)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        ordering = ["-created_at"]

    def __str__(self) -> str:
        return f"{self.buyer} offer on {self.listing}"


class Trade(models.Model):
    class Status(models.TextChoices):
        PENDING = "pending", "Pending"
        FUNDED = "funded", "Funded"
        DELIVERED = "delivered", "Delivered"
        COMPLETED = "completed", "Completed"
        DISPUTED = "disputed", "Disputed"

    offer = models.OneToOneField(Offer, on_delete=models.CASCADE, related_name="trade")
    on_chain_trade_id = models.CharField(max_length=128, unique=True)
    contract_address = models.CharField(max_length=128)
    status = models.CharField(max_length=12, choices=Status.choices, default=Status.PENDING)
    create_tx_hash = models.CharField(max_length=128, blank=True)
    deposit_tx_hash = models.CharField(max_length=128, blank=True)
    delivery_tx_hash = models.CharField(max_length=128, blank=True)
    confirmation_tx_hash = models.CharField(max_length=128, blank=True)
    dispute_tx_hash = models.CharField(max_length=128, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self) -> str:
        return self.on_chain_trade_id


class Feedback(models.Model):
    user = models.ForeignKey(
        settings.AUTH_USER_MODEL, on_delete=models.CASCADE, related_name="feedback"
    )
    trade = models.ForeignKey(Trade, on_delete=models.CASCADE, related_name="feedback")
    rating = models.PositiveSmallIntegerField(
        validators=[MinValueValidator(1), MaxValueValidator(5)]
    )
    comment = models.TextField(blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        unique_together = ["user", "trade"]
        ordering = ["-created_at"]

    def __str__(self) -> str:
        return f"{self.rating}/5 by {self.user}"


# Create your models here.
