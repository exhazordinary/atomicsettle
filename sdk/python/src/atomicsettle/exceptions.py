"""Exception types for AtomicSettle SDK."""

from __future__ import annotations

from typing import Optional


class AtomicSettleError(Exception):
    """Base exception for all AtomicSettle errors."""

    def __init__(
        self,
        message: str,
        code: Optional[str] = None,
        retryable: bool = False,
        retry_after_ms: Optional[int] = None,
    ):
        super().__init__(message)
        self.message = message
        self.code = code
        self.retryable = retryable
        self.retry_after_ms = retry_after_ms


class ConnectionError(AtomicSettleError):
    """Error connecting to coordinator."""

    def __init__(self, message: str, cause: Optional[Exception] = None):
        super().__init__(message, code="CONNECTION_ERROR", retryable=True, retry_after_ms=1000)
        self.cause = cause


class AuthenticationError(AtomicSettleError):
    """Authentication failed."""

    def __init__(self, message: str):
        super().__init__(message, code="AUTHENTICATION_ERROR", retryable=False)


class ValidationError(AtomicSettleError):
    """Request validation failed."""

    def __init__(self, message: str, field: Optional[str] = None):
        super().__init__(message, code="VALIDATION_ERROR", retryable=False)
        self.field = field


class SettlementError(AtomicSettleError):
    """Settlement processing error."""

    def __init__(
        self,
        message: str,
        settlement_id: Optional[str] = None,
        code: Optional[str] = None,
    ):
        super().__init__(message, code=code or "SETTLEMENT_ERROR", retryable=False)
        self.settlement_id = settlement_id


class InsufficientFundsError(SettlementError):
    """Insufficient funds for settlement."""

    def __init__(self, message: str, required: str, available: str):
        super().__init__(message, code="INSUFFICIENT_FUNDS")
        self.required = required
        self.available = available


class ParticipantOfflineError(SettlementError):
    """Target participant is offline."""

    def __init__(self, participant_id: str):
        super().__init__(
            f"Participant {participant_id} is offline",
            code="PARTICIPANT_OFFLINE",
        )
        self.participant_id = participant_id


class LockTimeoutError(SettlementError):
    """Lock acquisition timed out."""

    def __init__(self, settlement_id: str):
        super().__init__(
            f"Lock timeout for settlement {settlement_id}",
            settlement_id=settlement_id,
            code="LOCK_TIMEOUT",
        )


class TimeoutError(AtomicSettleError):
    """Operation timed out."""

    def __init__(self, operation: str, timeout_ms: int):
        super().__init__(
            f"{operation} timed out after {timeout_ms}ms",
            code="TIMEOUT",
            retryable=True,
            retry_after_ms=1000,
        )
        self.operation = operation
        self.timeout_ms = timeout_ms


class RateLimitedError(AtomicSettleError):
    """Rate limit exceeded."""

    def __init__(self, retry_after_ms: int):
        super().__init__(
            f"Rate limited, retry after {retry_after_ms}ms",
            code="RATE_LIMITED",
            retryable=True,
            retry_after_ms=retry_after_ms,
        )


class CoordinatorBusyError(AtomicSettleError):
    """Coordinator is overloaded."""

    def __init__(self, retry_after_ms: int):
        super().__init__(
            f"Coordinator busy, retry after {retry_after_ms}ms",
            code="COORDINATOR_BUSY",
            retryable=True,
            retry_after_ms=retry_after_ms,
        )
