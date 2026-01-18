"""AtomicSettle client for banks to integrate with the settlement network."""

from __future__ import annotations

import asyncio
import logging
from decimal import Decimal
from typing import AsyncIterator, Callable, Optional
from uuid import uuid4

from atomicsettle.exceptions import (
    AtomicSettleError,
    ConnectionError,
    ValidationError,
)
from atomicsettle.types import (
    Balance,
    Currency,
    FxInstruction,
    Money,
    Settlement,
    SettlementRequest,
    SettlementStatus,
)

logger = logging.getLogger(__name__)


class AtomicSettleClient:
    """
    Client for banks to integrate with AtomicSettle network.

    Example:
        >>> client = AtomicSettleClient(
        ...     participant_id="JPMORGAN_NY",
        ...     coordinator_url="https://coordinator.atomicsettle.network",
        ...     signing_key=load_key("private_key.pem")
        ... )
        >>>
        >>> # Connect to coordinator
        >>> await client.connect()
        >>>
        >>> # Send a settlement
        >>> settlement = await client.send(
        ...     to_participant="HSBC_LONDON",
        ...     amount=Decimal("10000000"),
        ...     currency=Currency.USD,
        ...     purpose="Trade settlement"
        ... )
        >>>
        >>> print(f"Settled in {settlement.duration_ms}ms")
        >>>
        >>> # Disconnect
        >>> await client.disconnect()
    """

    def __init__(
        self,
        participant_id: str,
        coordinator_url: str,
        signing_key: Optional[bytes] = None,
        cert_path: Optional[str] = None,
        key_path: Optional[str] = None,
        ca_cert_path: Optional[str] = None,
        timeout_ms: int = 30000,
    ):
        """
        Initialize AtomicSettle client.

        Args:
            participant_id: Unique identifier for this participant (e.g., "JPMORGAN_NY")
            coordinator_url: URL of the coordinator to connect to
            signing_key: Ed25519 private key bytes for signing messages
            cert_path: Path to client TLS certificate
            key_path: Path to client TLS private key
            ca_cert_path: Path to CA certificate for coordinator
            timeout_ms: Default timeout for operations in milliseconds
        """
        self.participant_id = participant_id
        self.coordinator_url = coordinator_url
        self.signing_key = signing_key
        self.cert_path = cert_path
        self.key_path = key_path
        self.ca_cert_path = ca_cert_path
        self.timeout_ms = timeout_ms

        self._connected = False
        self._incoming_handlers: list[Callable[[Settlement], None]] = []

    @property
    def is_connected(self) -> bool:
        """Check if connected to coordinator."""
        return self._connected

    async def connect(self) -> None:
        """
        Connect to the coordinator.

        Raises:
            ConnectionError: If connection fails
        """
        logger.info(
            f"Connecting to coordinator at {self.coordinator_url} as {self.participant_id}"
        )

        try:
            # In a real implementation:
            # 1. Establish gRPC/TLS connection
            # 2. Perform handshake
            # 3. Authenticate with certificate
            # 4. Start heartbeat loop

            self._connected = True
            logger.info(f"Connected to coordinator as {self.participant_id}")

        except Exception as e:
            raise ConnectionError(f"Failed to connect: {e}", cause=e)

    async def disconnect(self) -> None:
        """Disconnect from the coordinator."""
        if not self._connected:
            return

        logger.info(f"Disconnecting from coordinator")

        try:
            # In a real implementation:
            # 1. Send disconnect message
            # 2. Close connection
            # 3. Stop heartbeat loop

            self._connected = False
            logger.info("Disconnected from coordinator")

        except Exception as e:
            logger.error(f"Error during disconnect: {e}")
            self._connected = False

    async def send(
        self,
        to_participant: str,
        amount: Decimal | Money,
        currency: Optional[Currency] = None,
        purpose: str = "",
        remittance_info: Optional[str] = None,
        fx_instruction: Optional[FxInstruction] = None,
        idempotency_key: Optional[str] = None,
        wait_for_completion: bool = True,
        timeout_ms: Optional[int] = None,
    ) -> Settlement:
        """
        Send a settlement to another participant.

        Args:
            to_participant: Destination participant ID
            amount: Amount to send (Money object or Decimal)
            currency: Currency if amount is Decimal
            purpose: Purpose code (ISO 20022)
            remittance_info: Payment reference information
            fx_instruction: FX handling instruction for cross-currency
            idempotency_key: Unique key for duplicate detection
            wait_for_completion: Wait for settlement to complete
            timeout_ms: Timeout in milliseconds (overrides default)

        Returns:
            Settlement object with current status

        Raises:
            ValidationError: If request is invalid
            SettlementError: If settlement fails
            TimeoutError: If operation times out
        """
        self._ensure_connected()

        # Build Money object
        if isinstance(amount, Money):
            money = amount
        elif currency is not None:
            money = Money(value=amount, currency=currency)
        else:
            raise ValidationError("Currency is required when amount is Decimal", field="currency")

        # Generate idempotency key if not provided
        if idempotency_key is None:
            idempotency_key = str(uuid4())

        # Validate request
        if not to_participant:
            raise ValidationError("to_participant is required", field="to_participant")
        if money.value <= 0:
            raise ValidationError("Amount must be positive", field="amount")
        if to_participant == self.participant_id:
            raise ValidationError(
                "Cannot send settlement to self", field="to_participant"
            )

        logger.info(
            f"Sending settlement: {money} to {to_participant} "
            f"(idempotency_key={idempotency_key})"
        )

        request = SettlementRequest(
            to_participant=to_participant,
            amount=money,
            purpose=purpose,
            remittance_info=remittance_info,
            fx_instruction=fx_instruction,
            idempotency_key=idempotency_key,
        )

        # In a real implementation:
        # 1. Sign request
        # 2. Send to coordinator
        # 3. Wait for response
        # 4. If wait_for_completion, poll until final state

        # Placeholder: Return mock settlement
        raise NotImplementedError("Settlement sending not yet implemented")

    async def get_balance(self, currency: Currency) -> Balance:
        """
        Get current balance for a currency.

        Args:
            currency: Currency to query

        Returns:
            Balance object with available, locked, and pending amounts
        """
        self._ensure_connected()

        logger.debug(f"Querying balance for {currency.value}")

        # In a real implementation:
        # 1. Send balance query to coordinator
        # 2. Return response

        raise NotImplementedError("Balance query not yet implemented")

    async def get_balances(self) -> list[Balance]:
        """
        Get balances for all currencies.

        Returns:
            List of Balance objects
        """
        self._ensure_connected()

        # In a real implementation:
        # 1. Send balance query for all currencies
        # 2. Return responses

        raise NotImplementedError("Balance query not yet implemented")

    async def get_settlement(self, settlement_id: str) -> Settlement:
        """
        Get settlement by ID.

        Args:
            settlement_id: Settlement ID to query

        Returns:
            Settlement object
        """
        self._ensure_connected()

        logger.debug(f"Querying settlement {settlement_id}")

        # In a real implementation:
        # 1. Send settlement query to coordinator
        # 2. Return response

        raise NotImplementedError("Settlement query not yet implemented")

    async def list_settlements(
        self,
        status: Optional[SettlementStatus] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> list[Settlement]:
        """
        List settlements with optional filtering.

        Args:
            status: Filter by status
            limit: Maximum number of results
            offset: Offset for pagination

        Returns:
            List of Settlement objects
        """
        self._ensure_connected()

        logger.debug(f"Listing settlements (status={status}, limit={limit}, offset={offset})")

        # In a real implementation:
        # 1. Send list query to coordinator
        # 2. Return responses

        raise NotImplementedError("Settlement listing not yet implemented")

    def on_incoming(self, handler: Callable[[Settlement], None]) -> None:
        """
        Register handler for incoming settlements.

        Args:
            handler: Callback function that receives Settlement objects
        """
        self._incoming_handlers.append(handler)
        logger.debug(f"Registered incoming settlement handler")

    async def stream_settlements(self) -> AsyncIterator[Settlement]:
        """
        Stream incoming settlement notifications.

        Yields:
            Settlement objects as they arrive
        """
        self._ensure_connected()

        # In a real implementation:
        # 1. Subscribe to settlement notifications
        # 2. Yield settlements as they arrive

        raise NotImplementedError("Settlement streaming not yet implemented")

    async def __aenter__(self) -> AtomicSettleClient:
        """Async context manager entry."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit."""
        await self.disconnect()

    def _ensure_connected(self) -> None:
        """Ensure client is connected."""
        if not self._connected:
            raise ConnectionError("Not connected to coordinator")

    def _handle_incoming_settlement(self, settlement: Settlement) -> None:
        """Handle incoming settlement notification."""
        for handler in self._incoming_handlers:
            try:
                handler(settlement)
            except Exception as e:
                logger.error(f"Error in incoming settlement handler: {e}")


def load_key(path: str) -> bytes:
    """
    Load signing key from file.

    Args:
        path: Path to PEM-encoded private key

    Returns:
        Key bytes
    """
    with open(path, "rb") as f:
        return f.read()
