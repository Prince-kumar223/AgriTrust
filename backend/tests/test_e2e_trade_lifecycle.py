import os

import pytest


@pytest.mark.skipif(
    not os.environ.get("AGRITRUST_RUN_TESTNET_E2E"),
    reason="Set AGRITRUST_RUN_TESTNET_E2E=1 with funded testnet wallets to run.",
)
@pytest.mark.django_db
def test_full_trade_lifecycle_against_testnet():
    """Placeholder for the live testnet lifecycle.

    This is intentionally opt-in because it needs funded testnet accounts,
    Freighter-compatible signing, a deployed contract id, and an escrow asset
    contract id. The frontend and backend now expose the lifecycle boundaries
    this test should drive:
    listing API -> offer API -> create_trade -> Trade API -> deposit_payment
    -> record_deposit -> confirm_delivery -> record_delivery -> confirm_receipt
    -> record_confirmation.
    """

    raise NotImplementedError("Configure testnet wallets before enabling this test.")
