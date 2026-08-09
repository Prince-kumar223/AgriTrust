import pytest
from rest_framework.authtoken.models import Token
from rest_framework.test import APIClient

from agritrust.models import CropListing, Offer, Trade, User


@pytest.fixture
def api_client():
    return APIClient()


@pytest.fixture
def farmer(db):
    return User.objects.create_user(
        username="farmer",
        password="unused",
        role=User.Role.FARMER,
        wallet_address="GFARMER",
    )


@pytest.fixture
def buyer(db):
    return User.objects.create_user(
        username="buyer",
        password="unused",
        role=User.Role.BUYER,
        wallet_address="GBUYER",
    )


def authenticate(client, user):
    token, _ = Token.objects.get_or_create(user=user)
    client.credentials(HTTP_AUTHORIZATION=f"Token {token.key}")


@pytest.mark.django_db
def test_farmer_can_create_listing(api_client, farmer):
    authenticate(api_client, farmer)

    response = api_client.post(
        "/api/listings/",
        {
            "crop_type": "Wheat",
            "quantity": "10.00",
            "unit": "ton",
            "price_per_unit": "200.00",
            "description": "Grade A wheat",
        },
        format="json",
    )

    assert response.status_code == 201
    listing = CropListing.objects.get()
    assert listing.farmer == farmer
    assert listing.status == CropListing.Status.ACTIVE


@pytest.mark.django_db
def test_buyer_offer_flow_and_farmer_acceptance(api_client, farmer, buyer):
    listing = CropListing.objects.create(
        farmer=farmer,
        crop_type="Rice",
        quantity="5.00",
        unit="ton",
        price_per_unit="300.00",
    )
    authenticate(api_client, buyer)

    offer_response = api_client.post(
        "/api/offers/",
        {"listing": listing.id, "offered_price": "1450.00", "message": "Ready to buy"},
        format="json",
    )

    assert offer_response.status_code == 201
    offer = Offer.objects.get()
    assert offer.buyer == buyer
    listing.refresh_from_db()
    assert listing.status == CropListing.Status.OFFERED

    authenticate(api_client, farmer)
    accept_response = api_client.post(f"/api/offers/{offer.id}/accept/")

    assert accept_response.status_code == 200
    offer.refresh_from_db()
    listing.refresh_from_db()
    assert offer.status == Offer.Status.ACCEPTED
    assert listing.status == CropListing.Status.SOLD


@pytest.mark.django_db
def test_trade_state_sync(api_client, farmer, buyer):
    listing = CropListing.objects.create(
        farmer=farmer,
        crop_type="Tomato",
        quantity="12.00",
        unit="crate",
        price_per_unit="25.00",
    )
    offer = Offer.objects.create(
        listing=listing,
        buyer=buyer,
        offered_price="300.00",
        status=Offer.Status.ACCEPTED,
    )
    authenticate(api_client, farmer)

    create_response = api_client.post(
        "/api/trades/",
        {
            "offer": offer.id,
            "on_chain_trade_id": "trade-1",
            "contract_address": "CCONTRACT",
            "create_tx_hash": "tx-create",
        },
        format="json",
    )

    assert create_response.status_code == 201
    trade = Trade.objects.get()
    assert trade.status == Trade.Status.PENDING

    deposit_response = api_client.post(
        f"/api/trades/{trade.id}/record_deposit/",
        {"tx_hash": "tx-deposit"},
        format="json",
    )
    delivery_response = api_client.post(
        f"/api/trades/{trade.id}/record_delivery/",
        {"tx_hash": "tx-delivery"},
        format="json",
    )
    confirmation_response = api_client.post(
        f"/api/trades/{trade.id}/record_confirmation/",
        {"tx_hash": "tx-confirm"},
        format="json",
    )

    assert deposit_response.status_code == 200
    assert delivery_response.status_code == 200
    assert confirmation_response.status_code == 200
    trade.refresh_from_db()
    assert trade.status == Trade.Status.COMPLETED
    assert trade.deposit_tx_hash == "tx-deposit"
    assert trade.delivery_tx_hash == "tx-delivery"
    assert trade.confirmation_tx_hash == "tx-confirm"
