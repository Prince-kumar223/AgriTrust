from django.core.management.base import BaseCommand

from agritrust.chain import ChainStateUnavailable, get_on_chain_trade_status
from agritrust.models import Trade


class Command(BaseCommand):
    help = "Reconcile backend Trade.status values against the Soroban contract state."

    def add_arguments(self, parser):
        parser.add_argument("--once", action="store_true", help="Run one reconciliation pass.")

    def handle(self, *args, **options):
        updated = 0
        checked = 0
        for trade in Trade.objects.exclude(status=Trade.Status.COMPLETED):
            checked += 1
            try:
                chain_status = get_on_chain_trade_status(trade)
            except ChainStateUnavailable as exc:
                self.stderr.write(f"{trade.on_chain_trade_id}: {exc}")
                continue
            if chain_status != trade.status:
                trade.status = chain_status
                trade.save(update_fields=["status", "updated_at"])
                updated += 1
                self.stdout.write(f"{trade.on_chain_trade_id}: synced to {chain_status}")

        self.stdout.write(f"Checked {checked} trades, updated {updated}.")
