"""
AtomicSettle Python SDK

A client library for banks and financial institutions to integrate with
the AtomicSettle cross-border settlement network.

Example:
    >>> from atomicsettle import AtomicSettleClient, Currency
    >>> from decimal import Decimal
    >>>
    >>> client = AtomicSettleClient(
    ...     participant_id="BANK_A",
    ...     coordinator_url="https://coordinator.atomicsettle.network",
    ...     signing_key=load_key("private_key.pem")
    ... )
    >>>
    >>> settlement = await client.send(
    ...     to_participant="BANK_B",
    ...     amount=Decimal("1000000"),
    ...     currency=Currency.USD,
    ...     purpose="Trade settlement"
    ... )
    >>>
    >>> print(f"Settled in {settlement.duration_ms}ms")
"""

from atomicsettle.client import AtomicSettleClient
from atomicsettle.types import (
    Currency,
    Money,
    Balance,
    Settlement,
    SettlementStatus,
    SettlementRequest,
    FxInstruction,
    FxMode,
)
from atomicsettle.exceptions import (
    AtomicSettleError,
    ConnectionError,
    SettlementError,
    ValidationError,
    TimeoutError,
)

__version__ = "0.1.0"
__all__ = [
    # Client
    "AtomicSettleClient",
    # Types
    "Currency",
    "Money",
    "Balance",
    "Settlement",
    "SettlementStatus",
    "SettlementRequest",
    "FxInstruction",
    "FxMode",
    # Exceptions
    "AtomicSettleError",
    "ConnectionError",
    "SettlementError",
    "ValidationError",
    "TimeoutError",
]
