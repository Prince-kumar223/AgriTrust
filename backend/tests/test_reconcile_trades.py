import pytest
from django.core.management import call_command

from agritrust.models import CropListing, Offer, Trade, User


@pytest.mark.django_db
def test_reconcile_trades_updates_backend_status(monkeypatch):
    farmer = User.objects.create_user(
        username="farmer",
        password="unused",
        role=User.Role.FARMER,
        wallet_address="GFARMER",
    )
    buyer = User.objects.create_user(
        username="buyer",
        password="unused",
        role=User.Role.BUYER,
        wallet_address="GBUYER",
    )
    listing = CropListing.objects.create(
        farmer=farmer,
        crop_type="Rice",
        quantity="5.00",
        unit="ton",
        price_per_unit="300.00",
    )
    offer = Offer.objects.create(
        listing=listing,
        buyer=buyer,
        offered_price="1500.00",
        status=Offer.Status.ACCEPTED,
    )
    trade = Trade.objects.create(
        offer=offer,
        on_chain_trade_id="trade-reconcile",
        contract_address="CCONTRACT",
        create_tx_hash="tx-create",
    )
    monkeypatch.setattr(
        "agritrust.management.commands.reconcile_trades.get_on_chain_trade_status",
        lambda trade: Trade.Status.FUNDED,
    )

    call_command("reconcile_trades", "--once")

    trade.refresh_from_db()
    assert trade.status == Trade.Status.FUNDED
