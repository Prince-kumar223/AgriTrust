import json
import subprocess

from django.conf import settings

from .models import Trade

CHAIN_TO_BACKEND_STATUS = {
    "Pending": Trade.Status.PENDING,
    "Funded": Trade.Status.FUNDED,
    "Delivered": Trade.Status.DELIVERED,
    "Completed": Trade.Status.COMPLETED,
    "Disputed": Trade.Status.DISPUTED,
}


class ChainStateUnavailable(RuntimeError):
    pass


def parse_trade_state(payload: str) -> str:
    for chain_state, backend_state in CHAIN_TO_BACKEND_STATUS.items():
        if chain_state in payload:
            return backend_state
    raise ChainStateUnavailable(f"Could not parse trade state from chain response: {payload}")


def get_on_chain_trade_status(trade: Trade) -> str:
    command = [
        settings.STELLAR_CLI_BIN,
        "contract",
        "invoke",
        "--network",
        settings.STELLAR_NETWORK,
        "--id",
        trade.contract_address,
        "--",
        "get_trade",
        "--trade_id",
        trade.on_chain_trade_id,
    ]
    try:
        result = subprocess.run(
            command,
            check=True,
            capture_output=True,
            text=True,
            timeout=settings.STELLAR_CLI_TIMEOUT_SECONDS,
        )
    except (subprocess.SubprocessError, OSError) as exc:
        raise ChainStateUnavailable(str(exc)) from exc

    output = result.stdout.strip()
    try:
        parsed = json.loads(output)
    except json.JSONDecodeError:
        parsed = output
    return parse_trade_state(json.dumps(parsed))
