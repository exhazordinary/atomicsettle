"""Type definitions for AtomicSettle SDK."""

from __future__ import annotations

from datetime import datetime
from decimal import Decimal
from enum import Enum
from typing import Optional
from uuid import UUID

from pydantic import BaseModel, Field


class Currency(str, Enum):
    """ISO 4217 currency codes."""

    USD = "USD"
    EUR = "EUR"
    GBP = "GBP"
    JPY = "JPY"
    CHF = "CHF"
    AUD = "AUD"
    CAD = "CAD"
    CNY = "CNY"
    HKD = "HKD"
    SGD = "SGD"
    MYR = "MYR"
    INR = "INR"

    @property
    def decimal_places(self) -> int:
        """Get standard decimal places for this currency."""
        if self in (Currency.JPY,):
            return 0
        return 2


class SettlementStatus(str, Enum):
    """Settlement lifecycle status."""

    INITIATED = "INITIATED"
    VALIDATED = "VALIDATED"
    PENDING_REVIEW = "PENDING_REVIEW"
    LOCKING = "LOCKING"
    LOCKED = "LOCKED"
    COMMITTING = "COMMITTING"
    COMMITTED = "COMMITTED"
    SETTLED = "SETTLED"
    REJECTED = "REJECTED"
    FAILED = "FAILED"

    @property
    def is_final(self) -> bool:
        """Check if this is a final state."""
        return self in (
            SettlementStatus.SETTLED,
            SettlementStatus.REJECTED,
            SettlementStatus.FAILED,
        )

    @property
    def is_success(self) -> bool:
        """Check if this is a successful final state."""
        return self == SettlementStatus.SETTLED


class FxMode(str, Enum):
    """FX execution mode."""

    AT_SOURCE = "AT_SOURCE"
    AT_DESTINATION = "AT_DESTINATION"
    AT_COORDINATOR = "AT_COORDINATOR"


class Money(BaseModel):
    """Monetary amount with currency."""

    value: Decimal
    currency: Currency

    def __str__(self) -> str:
        return f"{self.value} {self.currency.value}"

    def __add__(self, other: Money) -> Money:
        if self.currency != other.currency:
            raise ValueError(f"Cannot add {self.currency} and {other.currency}")
        return Money(value=self.value + other.value, currency=self.currency)

    def __sub__(self, other: Money) -> Money:
        if self.currency != other.currency:
            raise ValueError(f"Cannot subtract {self.currency} and {other.currency}")
        return Money(value=self.value - other.value, currency=self.currency)

    @classmethod
    def usd(cls, amount: Decimal | str | int | float) -> Money:
        """Create USD amount."""
        return cls(value=Decimal(str(amount)), currency=Currency.USD)

    @classmethod
    def eur(cls, amount: Decimal | str | int | float) -> Money:
        """Create EUR amount."""
        return cls(value=Decimal(str(amount)), currency=Currency.EUR)


class Balance(BaseModel):
    """Account balance information."""

    currency: Currency
    available: Decimal = Field(description="Available for settlement")
    locked: Decimal = Field(description="Reserved for pending settlements")
    pending_in: Decimal = Field(description="Incoming (not yet final)")
    pending_out: Decimal = Field(description="Outgoing (not yet final)")

    @property
    def total(self) -> Decimal:
        """Get total balance (available + locked)."""
        return self.available + self.locked

    @property
    def projected(self) -> Decimal:
        """Get projected balance including pending."""
        return self.total + self.pending_in - self.pending_out


class FxInstruction(BaseModel):
    """FX instruction for cross-currency settlements."""

    mode: FxMode = FxMode.AT_COORDINATOR
    target_currency: Optional[Currency] = None
    rate_reference: Optional[str] = None


class FxRate(BaseModel):
    """FX rate between two currencies."""

    base_currency: Currency
    quote_currency: Currency
    bid: Decimal
    ask: Decimal
    mid: Decimal
    quoted_at: datetime
    valid_until: datetime
    source: str

    @property
    def spread_bps(self) -> Decimal:
        """Get spread in basis points."""
        return ((self.ask - self.bid) / self.mid) * 10000


class SettlementTiming(BaseModel):
    """Timing metrics for a settlement."""

    initiated_at: datetime
    validated_at: Optional[datetime] = None
    locked_at: Optional[datetime] = None
    committed_at: Optional[datetime] = None
    settled_at: Optional[datetime] = None
    failed_at: Optional[datetime] = None

    @property
    def total_duration_ms(self) -> Optional[int]:
        """Get total duration in milliseconds."""
        if self.settled_at:
            delta = self.settled_at - self.initiated_at
            return int(delta.total_seconds() * 1000)
        return None


class SettlementLeg(BaseModel):
    """A single leg of a settlement."""

    leg_number: int
    from_participant: str
    from_account: str
    to_participant: str
    to_account: str
    amount: Money
    fx_instruction: Optional[FxInstruction] = None
    converted_amount: Optional[Money] = None


class SettlementFailure(BaseModel):
    """Settlement failure information."""

    code: str
    message: str
    failed_leg: Optional[int] = None
    failed_at: datetime


class Settlement(BaseModel):
    """A complete settlement."""

    id: UUID
    idempotency_key: str
    status: SettlementStatus
    legs: list[SettlementLeg]
    timing: SettlementTiming
    fx_rate: Optional[FxRate] = None
    failure: Optional[SettlementFailure] = None
    metadata: dict[str, str] = Field(default_factory=dict)

    @property
    def is_complete(self) -> bool:
        """Check if settlement is complete (success or failure)."""
        return self.status.is_final

    @property
    def is_success(self) -> bool:
        """Check if settlement completed successfully."""
        return self.status.is_success

    @property
    def duration_ms(self) -> Optional[int]:
        """Get total duration in milliseconds."""
        return self.timing.total_duration_ms

    @property
    def total_amount(self) -> Optional[Money]:
        """Get total settlement amount."""
        if not self.legs:
            return None
        return self.legs[0].amount


class SettlementRequest(BaseModel):
    """Request to create a settlement."""

    to_participant: str
    amount: Money
    purpose: str
    remittance_info: Optional[str] = None
    fx_instruction: Optional[FxInstruction] = None
    idempotency_key: Optional[str] = None
    metadata: dict[str, str] = Field(default_factory=dict)
