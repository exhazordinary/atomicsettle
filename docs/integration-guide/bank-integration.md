# Bank Integration Guide

This guide walks through integrating a bank with the AtomicSettle network.

## Overview

Bank integration involves:

1. **Onboarding**: Register with a coordinator operator
2. **Technical Setup**: Configure SDK and certificates
3. **Testing**: Validate in sandbox environment
4. **Production**: Go live with limited volume

## Prerequisites

Before starting:

- [ ] Signed agreement with coordinator operator
- [ ] Received participant credentials
- [ ] IT team familiar with REST/gRPC APIs
- [ ] Test environment available

## Step 1: Obtain Credentials

Contact your coordinator operator to receive:

1. **Participant ID**: Your unique identifier (e.g., `YOURBANK_NY`)
2. **Client Certificate**: X.509 certificate for authentication
3. **Private Key**: Keep this secure!
4. **Coordinator CA**: To verify coordinator identity
5. **Sandbox URL**: For testing

## Step 2: Install SDK

Choose the SDK for your platform:

### Python

```bash
pip install atomicsettle
```

### Java

```xml
<dependency>
    <groupId>network.atomicsettle</groupId>
    <artifactId>atomicsettle-client</artifactId>
    <version>0.1.0</version>
</dependency>
```

### TypeScript

```bash
npm install @atomicsettle/client
```

## Step 3: Configure Connection

### Python Example

```python
from atomicsettle import AtomicSettleClient

client = AtomicSettleClient(
    participant_id="YOURBANK_NY",
    coordinator_url="https://sandbox.coordinator.atomicsettle.network",
    cert_path="/path/to/client.crt",
    key_path="/path/to/client.key",
    ca_cert_path="/path/to/coordinator-ca.crt"
)
```

### Java Example

```java
AtomicSettleClient client = AtomicSettleClient.builder()
    .participantId("YOURBANK_NY")
    .coordinatorUrl("https://sandbox.coordinator.atomicsettle.network")
    .certPath("/path/to/client.crt")
    .keyPath("/path/to/client.key")
    .caCertPath("/path/to/coordinator-ca.crt")
    .build();
```

## Step 4: Implement Core Flows

### Sending Settlements

```python
import asyncio
from decimal import Decimal
from atomicsettle import AtomicSettleClient, Currency

async def send_payment():
    async with AtomicSettleClient(...) as client:
        settlement = await client.send(
            to_participant="OTHERBANK_LONDON",
            amount=Decimal("1000000.00"),
            currency=Currency.USD,
            purpose="TRADE",  # ISO 20022 purpose code
            remittance_info="Invoice INV-2024-001"
        )

        print(f"Settlement ID: {settlement.id}")
        print(f"Status: {settlement.status}")

        if settlement.is_success:
            print(f"Settled in {settlement.duration_ms}ms")
        else:
            print(f"Failed: {settlement.failure.message}")

asyncio.run(send_payment())
```

### Receiving Settlements

```python
async def handle_incoming(settlement):
    """Called when we receive a settlement."""
    print(f"Received {settlement.total_amount} from {settlement.legs[0].from_participant}")

    # Update your core banking system
    await your_core_banking.credit_account(
        account=map_to_internal_account(settlement),
        amount=settlement.total_amount,
        reference=str(settlement.id)
    )

# Register handler
client.on_incoming(handle_incoming)
```

### Handling Lock Requests

When you're the source of a settlement, the coordinator will request a lock:

```python
async def handle_lock_request(lock_id, settlement_id, amount):
    """Called when coordinator requests to lock funds."""
    # Check if funds are available
    if await your_core_banking.can_lock(amount):
        # Lock in your system
        await your_core_banking.create_hold(
            amount=amount,
            reference=str(lock_id)
        )
        return True  # Approve lock
    else:
        return False  # Reject lock

client.on_lock_request(handle_lock_request)
```

## Step 5: Implement Reconciliation

Daily reconciliation ensures your records match the coordinator:

```python
async def daily_reconciliation():
    # Get settlements for the day
    settlements = await client.list_settlements(
        from_date=yesterday,
        to_date=today
    )

    for settlement in settlements:
        # Compare with your records
        local_record = await your_db.find_settlement(settlement.id)

        if local_record is None:
            # Missing locally - investigate
            log.warning(f"Settlement {settlement.id} missing locally")
        elif local_record.status != settlement.status:
            # Status mismatch - investigate
            log.warning(f"Status mismatch for {settlement.id}")

    # Compare balances
    for currency in [Currency.USD, Currency.EUR, Currency.GBP]:
        coordinator_balance = await client.get_balance(currency)
        local_balance = await your_db.get_position(currency)

        if coordinator_balance.total != local_balance:
            log.warning(f"Balance mismatch for {currency}")
```

## Step 6: Testing

### Sandbox Testing

1. Connect to sandbox environment
2. Use test counterparties
3. Send test settlements
4. Verify handling of failures

### Test Cases

| Test Case | Expected Result |
|-----------|-----------------|
| Simple settlement | Completes in < 3s |
| Insufficient funds | Rejected with error |
| Counterparty offline | Times out after 30s |
| FX settlement | Converts at locked rate |

## Step 7: Go Live

### Pre-Production Checklist

- [ ] All test cases passing
- [ ] Reconciliation process implemented
- [ ] Monitoring and alerting configured
- [ ] Runbook documented
- [ ] On-call rotation established
- [ ] Regulatory approvals obtained

### Gradual Rollout

1. **Week 1**: 10 settlements/day
2. **Week 2**: 100 settlements/day
3. **Week 3**: 1,000 settlements/day
4. **Week 4+**: Full volume

## Troubleshooting

### Connection Issues

```python
# Enable debug logging
import logging
logging.getLogger('atomicsettle').setLevel(logging.DEBUG)
```

### Settlement Failures

Check the failure code:

| Code | Meaning | Action |
|------|---------|--------|
| INSUFFICIENT_FUNDS | Not enough balance | Check position |
| PARTICIPANT_OFFLINE | Counterparty down | Retry later |
| LOCK_TIMEOUT | Lock acquisition failed | Check local system |
| COMPLIANCE_REJECTED | Failed compliance | Review transaction |

## Support

- Documentation: https://docs.atomicsettle.network
- Support: support@atomicsettle.network
- Slack: atomicsettle.slack.com
