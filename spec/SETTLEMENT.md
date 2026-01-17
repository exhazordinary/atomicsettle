# AtomicSettle Settlement Lifecycle Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-01-17

## Overview

This document provides a detailed specification of the settlement lifecycle, including state transitions, timing constraints, and recovery procedures.

## Table of Contents

1. [Settlement State Machine](#1-settlement-state-machine)
2. [Phase Details](#2-phase-details)
3. [Lock Management](#3-lock-management)
4. [Atomic Commit](#4-atomic-commit)
5. [Failure Recovery](#5-failure-recovery)
6. [Multi-Leg Settlements](#6-multi-leg-settlements)
7. [Netting Integration](#7-netting-integration)

---

## 1. Settlement State Machine

### 1.1 Complete State Diagram

```
                                    ┌─────────────────┐
                                    │    RECEIVED     │
                                    │   (transient)   │
                                    └────────┬────────┘
                                             │
                                    ┌────────▼────────┐
                              ┌─────│    INITIATED    │─────┐
                              │     └────────┬────────┘     │
                              │              │              │
                      validation        validation     compliance
                       failed            passed          hold
                              │              │              │
                              ▼              ▼              ▼
                    ┌─────────────┐ ┌───────────────┐ ┌──────────┐
                    │  REJECTED   │ │   VALIDATED   │ │ PENDING_ │
                    │  (final)    │ │               │ │ REVIEW   │
                    └─────────────┘ └───────┬───────┘ └────┬─────┘
                                            │              │
                                            │         manual review
                                            │         approve/reject
                                            │              │
                                    ┌───────▼───────┐      │
                              ┌─────│    LOCKING    │◄─────┘
                              │     └───────┬───────┘
                              │             │
                          lock          all locks
                          failed        acquired
                              │             │
                              ▼             ▼
                    ┌─────────────┐ ┌───────────────┐
                    │   FAILED    │ │    LOCKED     │
                    │  (final)    │ └───────┬───────┘
                    └─────────────┘         │
                           ▲                │
                           │           coordinator
                           │            commits
                       timeout             │
                       or error            ▼
                           │     ┌─────────────────┐
                           └─────│   COMMITTING    │
                                 └────────┬────────┘
                                          │
                                     committed
                                          │
                                          ▼
                                 ┌─────────────────┐
                                 │    COMMITTED    │
                                 └────────┬────────┘
                                          │
                                   all parties
                                   acknowledged
                                          │
                                          ▼
                                 ┌─────────────────┐
                                 │    SETTLED      │
                                 │    (final)      │
                                 └─────────────────┘
```

### 1.2 State Definitions

| State | Type | Duration | Description |
|-------|------|----------|-------------|
| RECEIVED | Transient | < 10ms | Message received, awaiting processing |
| INITIATED | Stable | < 100ms | Request validated, processing started |
| VALIDATED | Stable | < 500ms | All checks passed, ready for locking |
| PENDING_REVIEW | Stable | Hours/Days | Manual compliance review required |
| LOCKING | Transient | < 10s | Acquiring locks from participants |
| LOCKED | Stable | < 30s | All locks acquired, awaiting commit |
| COMMITTING | Transient | < 200ms | Executing atomic commit |
| COMMITTED | Stable | < 60s | Committed, awaiting acknowledgments |
| SETTLED | Final | - | Complete, all parties acknowledged |
| REJECTED | Final | - | Could not process request |
| FAILED | Final | - | Failed after partial processing |

### 1.3 Invariants

1. **Forward-only progression**: State can only move forward, never backward (except PENDING_REVIEW → REJECTED)
2. **Single active settlement per idempotency key**: Duplicate requests return existing result
3. **Lock-before-commit**: COMMITTED can only be reached from LOCKED
4. **No partial commits**: All legs commit atomically or none do
5. **Bounded state duration**: No stable state exceeds its timeout

---

## 2. Phase Details

### 2.1 Initiation Phase

**Entry**: SettleRequest received
**Exit**: VALIDATED or REJECTED

#### 2.1.1 Validation Steps

```
1. Message Format Validation
   ├── Envelope present and complete
   ├── All required fields present
   ├── Field formats valid (UUIDs, amounts, currencies)
   └── Signature valid

2. Participant Validation
   ├── Sender registered and active
   ├── Receiver registered and active
   ├── Both participants connected (or PENDING_REVIEW if offline)
   └── Routing possible between participants

3. Amount Validation
   ├── Amount positive and within system limits
   ├── Currency supported
   ├── FX pair supported (if cross-currency)
   └── Decimal precision acceptable

4. Compliance Pre-checks
   ├── Sanctions screening (sender, receiver)
   ├── Transaction limits check
   ├── Velocity checks (fraud prevention)
   └── Jurisdiction restrictions

5. Credit Check
   ├── Sender has sufficient position
   ├── Credit limits not exceeded
   └── No account holds/freezes
```

#### 2.1.2 FX Rate Lock

If cross-currency settlement:

```python
def lock_fx_rate(request):
    if request.fx_instruction.locked_rate:
        # Validate pre-locked rate is still valid
        validate_rate_freshness(request.fx_instruction.locked_rate)
        return request.fx_instruction.locked_rate
    else:
        # Fetch and lock current rate
        rate = fx_engine.get_rate(
            request.amount.currency,
            request.fx_instruction.target_currency
        )
        rate.locked_until = now() + RATE_LOCK_DURATION  # 30s
        return rate
```

### 2.2 Locking Phase

**Entry**: Settlement validated
**Exit**: LOCKED or FAILED

#### 2.2.1 Lock Acquisition Protocol

```
For each leg in settlement.legs:
    1. Send LOCK_REQUEST to leg.source_participant
       - Include lock_id, settlement_id, amount, expiry
       - Coordinator signs request

    2. Await LOCK_RESPONSE
       - If ACQUIRED: Record lock, continue
       - If FAILED: Abort, release acquired locks
       - If TIMEOUT: Retry up to 3 times, then abort

    3. Verify lock
       - Participant signature valid
       - Lock expiry matches request
       - Amount matches request
```

#### 2.2.2 Lock Ordering

To prevent deadlocks in multi-leg settlements, locks are acquired in deterministic order:

```python
def get_lock_order(legs):
    # Sort by participant ID to ensure consistent ordering
    return sorted(legs, key=lambda l: (l.source_participant, l.leg_number))
```

#### 2.2.3 Participant Lock Implementation

At the participant side:

```python
class LockManager:
    def acquire_lock(self, request: LockRequest) -> LockResponse:
        with self.db.transaction():
            # Check available balance
            available = self.get_available_balance(request.account)
            if available < request.amount:
                return LockResponse(
                    status=FAILED,
                    failure=InsufficientFunds(available=available)
                )

            # Create lock record
            lock = Lock(
                id=request.lock_id,
                settlement_id=request.settlement_id,
                account=request.account,
                amount=request.amount,
                expires_at=request.expires_at,
                status=ACTIVE
            )
            self.db.insert(lock)

            # Update account balance
            self.db.execute("""
                UPDATE accounts
                SET available_balance = available_balance - :amount,
                    locked_balance = locked_balance + :amount
                WHERE account_id = :account_id
            """, amount=request.amount, account_id=request.account)

            return LockResponse(
                status=ACQUIRED,
                locked_at=now(),
                actual_expires_at=request.expires_at
            )
```

### 2.3 Commit Phase

**Entry**: All locks acquired
**Exit**: COMMITTED

#### 2.3.1 Atomic Commit Protocol

The coordinator executes the atomic commit within a single database transaction:

```python
async def atomic_commit(self, settlement: Settlement) -> CommitResult:
    async with self.db.transaction(isolation=SERIALIZABLE):
        # 1. Verify all locks still valid
        for leg in settlement.legs:
            lock = await self.get_lock(leg.lock_id)
            if lock.status != ACTIVE or lock.expires_at < now():
                raise LockExpiredError(leg.lock_id)

        # 2. Execute transfers on coordinator ledger
        for leg in settlement.legs:
            # Debit source
            await self.ledger.debit(
                account=leg.source_account,
                amount=leg.amount,
                settlement_id=settlement.id,
                leg_number=leg.leg_number
            )

            # Credit destination
            await self.ledger.credit(
                account=leg.destination_account,
                amount=leg.converted_amount,  # After FX if applicable
                settlement_id=settlement.id,
                leg_number=leg.leg_number
            )

        # 3. Mark settlement as committed
        settlement.status = COMMITTED
        settlement.committed_at = now()
        await self.db.update(settlement)

        # 4. Mark locks as consumed
        for leg in settlement.legs:
            await self.db.execute("""
                UPDATE locks SET status = 'CONSUMED' WHERE id = :id
            """, id=leg.lock_id)

        # Transaction commits here
        return CommitResult(success=True, committed_at=settlement.committed_at)
```

#### 2.3.2 Point of No Return

Once the commit transaction succeeds:
- Settlement CANNOT be rolled back at protocol level
- Funds have moved on coordinator ledger
- Participants MUST update their local ledgers

### 2.4 Settlement Phase

**Entry**: Committed
**Exit**: SETTLED

#### 2.4.1 Notification and Acknowledgment

```
1. Coordinator sends SETTLEMENT_NOTIFICATION to all participants
   - Contains full settlement details
   - Signed by coordinator

2. Participants acknowledge
   - Update local ledger
   - Send SETTLEMENT_ACK to coordinator
   - Include local reference number

3. Coordinator marks SETTLED when:
   - All participants have acknowledged, OR
   - Acknowledgment timeout (60s) reached

Note: Acknowledgment is fire-and-forget. Settlement is
final even if a participant fails to acknowledge.
```

#### 2.4.2 Participant Reconciliation

At the participant:

```python
async def handle_settlement_notification(self, notification):
    settlement = notification.settlement

    # Find matching local transaction
    local_tx = await self.find_pending_transaction(
        settlement_id=settlement.id
    )

    # Update local ledger
    async with self.db.transaction():
        if local_tx:
            # Update existing pending transaction
            local_tx.status = 'SETTLED'
            local_tx.settled_at = settlement.settled_at
            local_tx.coordinator_reference = settlement.id
            await self.db.update(local_tx)
        else:
            # Create new record (incoming settlement)
            await self.create_settlement_record(settlement)

        # Release local lock
        await self.release_lock(settlement.id)

    # Send acknowledgment
    await self.send_settlement_ack(settlement.id)

    # Emit event for downstream systems
    await self.event_bus.publish(SettlementCompleted(settlement))
```

---

## 3. Lock Management

### 3.1 Lock Properties

| Property | Value | Notes |
|----------|-------|-------|
| Default duration | 30 seconds | From acquisition to expiry |
| Maximum duration | 60 seconds | Extended locks for slow networks |
| Minimum duration | 5 seconds | Below this, timeout risk too high |
| Extension allowed | Yes, once | Additional 30s if coordinator requests |

### 3.2 Lock Lifecycle

```
                     ┌──────────────┐
                     │   PENDING    │
                     └──────┬───────┘
                            │
                    acquire request
                            │
               ┌────────────┼────────────┐
               │            │            │
           success       failed       timeout
               │            │            │
               ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │  ACTIVE  │ │  FAILED  │ │ EXPIRED  │
        └────┬─────┘ └──────────┘ └──────────┘
             │
    ┌────────┼────────┐
    │        │        │
 commit   timeout   abort
    │        │        │
    ▼        ▼        ▼
┌────────┐ ┌────────┐ ┌────────┐
│CONSUMED│ │EXPIRED │ │RELEASED│
└────────┘ └────────┘ └────────┘
```

### 3.3 Lock Contention

When multiple settlements compete for the same funds:

```python
class LockManager:
    async def handle_contention(self, account, amount, requests):
        # Strategy: First-come-first-served with priority override

        # Sort by (priority DESC, timestamp ASC)
        sorted_requests = sorted(
            requests,
            key=lambda r: (-r.priority, r.timestamp)
        )

        available = await self.get_available_balance(account)

        results = []
        for request in sorted_requests:
            if available >= request.amount:
                # Grant lock
                lock = await self.create_lock(request)
                available -= request.amount
                results.append((request, lock))
            else:
                # Reject - insufficient funds after prior locks
                results.append((request, InsufficientFunds()))

        return results
```

### 3.4 Automatic Lock Cleanup

Background process to handle expired locks:

```python
async def cleanup_expired_locks():
    while True:
        expired = await db.query("""
            SELECT * FROM locks
            WHERE status = 'ACTIVE'
            AND expires_at < NOW()
            FOR UPDATE SKIP LOCKED
        """)

        for lock in expired:
            async with db.transaction():
                # Release locked funds
                await db.execute("""
                    UPDATE accounts
                    SET available_balance = available_balance + :amount,
                        locked_balance = locked_balance - :amount
                    WHERE account_id = :account_id
                """, amount=lock.amount, account_id=lock.account_id)

                # Mark lock expired
                lock.status = 'EXPIRED'
                await db.update(lock)

                # Notify coordinator if connected
                await notify_coordinator_lock_expired(lock)

        await sleep(1)  # Run every second
```

---

## 4. Atomic Commit

### 4.1 Two-Phase Commit Variant

AtomicSettle uses a coordinator-driven two-phase commit:

**Phase 1: Prepare (Lock Acquisition)**
- Coordinator requests locks from all participants
- Each participant either grants lock or fails
- If any participant fails, abort entire settlement

**Phase 2: Commit (Execution)**
- Coordinator executes atomic transfer on its ledger
- Single transaction ensures atomicity
- Participants notified post-commit

### 4.2 Coordinator Ledger

The coordinator maintains an authoritative double-entry ledger:

```sql
CREATE TABLE journal_entries (
    id BIGSERIAL PRIMARY KEY,
    settlement_id UUID NOT NULL,
    leg_number INT NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    entry_type VARCHAR(10) NOT NULL,  -- 'DEBIT' or 'CREDIT'
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    balance_after DECIMAL(20, 8) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    CONSTRAINT balanced_entries CHECK (
        -- Every settlement must have balanced debits/credits
    )
);

CREATE TABLE account_balances (
    account_id VARCHAR(64) PRIMARY KEY,
    currency VARCHAR(3) NOT NULL,
    balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    locked_balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    version BIGINT NOT NULL DEFAULT 0,  -- Optimistic locking
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### 4.3 Commit Guarantees

1. **Durability**: Committed transactions survive coordinator restarts
2. **Isolation**: Concurrent settlements don't interfere
3. **Consistency**: Ledger always balances (debits = credits)
4. **Atomicity**: All legs commit or none do

---

## 5. Failure Recovery

### 5.1 Failure Matrix

| Failure Point | Settlement State | Recovery Action |
|---------------|------------------|-----------------|
| Before validation | INITIATED | Reject, no locks held |
| During validation | INITIATED | Reject, no locks held |
| Lock acquisition | LOCKING | Release acquired locks, fail |
| Lock timeout | LOCKED | Release all locks, fail |
| During commit | COMMITTING | See section 5.3 |
| After commit | COMMITTED | Retry notification, mark settled |

### 5.2 Participant Failure During Lock

```python
async def handle_participant_failure_during_lock(settlement, failed_participant):
    # 1. Stop waiting for failed participant
    cancel_pending_lock_request(settlement.id, failed_participant)

    # 2. Release all acquired locks
    for lock in settlement.acquired_locks:
        await release_lock(lock)

    # 3. Mark settlement failed
    settlement.status = FAILED
    settlement.failure_reason = ParticipantUnavailable(failed_participant)
    await db.update(settlement)

    # 4. Notify initiator
    await notify_settlement_failed(settlement)
```

### 5.3 Coordinator Failure During Commit

This is the critical failure scenario. Recovery depends on whether the commit transaction completed:

```python
async def recover_committing_settlement(settlement):
    # Check if commit actually succeeded
    committed = await check_commit_status(settlement.id)

    if committed:
        # Commit succeeded - continue to notification
        settlement.status = COMMITTED
        await proceed_to_notification(settlement)
    else:
        # Commit did not succeed - need to re-evaluate
        locks_valid = await verify_all_locks_valid(settlement)

        if locks_valid:
            # Retry commit
            await atomic_commit(settlement)
        else:
            # Locks expired - fail settlement
            settlement.status = FAILED
            settlement.failure_reason = "Commit failed - locks expired during recovery"
            await db.update(settlement)
            await notify_settlement_failed(settlement)
```

### 5.4 Recovery on Coordinator Startup

```python
async def coordinator_recovery():
    # 1. Recover Raft state
    await raft.recover()

    # 2. Find settlements in non-final states
    pending = await db.query("""
        SELECT * FROM settlements
        WHERE status NOT IN ('SETTLED', 'REJECTED', 'FAILED')
        ORDER BY created_at
    """)

    for settlement in pending:
        match settlement.status:
            case 'INITIATED' | 'VALIDATED':
                # Check if still valid, continue or timeout
                if settlement.created_at < now() - VALIDATION_TIMEOUT:
                    await timeout_settlement(settlement)
                else:
                    await resume_validation(settlement)

            case 'LOCKING':
                # Re-check lock status with participants
                await reconcile_locks(settlement)

            case 'LOCKED':
                # Check lock validity, commit if still valid
                if await all_locks_valid(settlement):
                    await atomic_commit(settlement)
                else:
                    await fail_settlement(settlement, "Locks expired during recovery")

            case 'COMMITTING':
                # Critical - determine commit outcome
                await recover_committing_settlement(settlement)

            case 'COMMITTED':
                # Retry notifications
                await retry_notifications(settlement)
```

---

## 6. Multi-Leg Settlements

### 6.1 Use Cases

1. **Payment-vs-Payment (PvP)**: Simultaneous exchange of currencies
2. **Delivery-vs-Payment (DvP)**: Security transfer against payment
3. **Multi-party**: One sender, multiple receivers
4. **Chain**: A→B→C where B is intermediary

### 6.2 Leg Ordering

All legs are committed atomically, but locks are acquired sequentially to prevent deadlock:

```python
def plan_multi_leg_settlement(settlement):
    # Sort legs for deterministic lock ordering
    legs = sorted(settlement.legs, key=lambda l: l.source_participant)

    # Validate cross-leg consistency
    validate_legs_balance(legs)  # Total in = Total out (per currency)

    return LockPlan(
        legs=legs,
        timeout=calculate_timeout(len(legs))  # More legs = more time
    )
```

### 6.3 Partial Failure

If any leg fails to lock, all legs are rolled back:

```python
async def execute_multi_leg_locks(settlement, legs):
    acquired_locks = []

    try:
        for leg in legs:
            lock = await acquire_lock(leg)
            if lock.status != ACQUIRED:
                raise LockFailedError(leg, lock.failure)
            acquired_locks.append(lock)

        return acquired_locks

    except LockFailedError as e:
        # Rollback all acquired locks
        for lock in acquired_locks:
            await release_lock(lock)
        raise
```

---

## 7. Netting Integration

### 7.1 Netting Window

Settlements can be held briefly for netting:

```python
class NettingEngine:
    WINDOW_SIZE = 100  # milliseconds

    async def submit_for_netting(self, settlement):
        # Add to current netting window
        window = self.get_current_window()
        window.add(settlement)

        # If window is closing, process
        if window.should_close():
            await self.process_window(window)
```

### 7.2 Netting Calculation

```python
def calculate_net_positions(settlements):
    # Build position matrix
    positions = defaultdict(lambda: defaultdict(Decimal))

    for s in settlements:
        for leg in s.legs:
            # Net by (source, destination, currency)
            key = (leg.source_participant, leg.destination_participant)
            positions[leg.currency][key] += leg.amount

    # Calculate net amounts
    net_settlements = []
    for currency, flows in positions.items():
        for (p1, p2), amount in flows.items():
            reverse_amount = flows.get((p2, p1), Decimal(0))
            net = amount - reverse_amount

            if net > 0:
                net_settlements.append(NetSettlement(
                    source=p1,
                    destination=p2,
                    currency=currency,
                    gross_amount=amount,
                    net_amount=net,
                    netting_savings=reverse_amount
                ))

    return net_settlements
```

### 7.3 Netting Benefits

Example with 4 settlements:
```
A → B: $100
B → A: $80
A → B: $50
B → A: $30

Gross: $260
Net: A → B: $40
Savings: 85%
```

---

## Appendix A: Timing Diagrams

### Successful 2-Party Settlement

```
Time    Sender(A)           Coordinator           Receiver(B)
─────────────────────────────────────────────────────────────
 0ms    SETTLE_REQ ────────►
                            validate
50ms                        ◄──── validated
                            LOCK_REQ ──────────────────────►
100ms                                              acquire lock
150ms                       ◄───────────────── LOCK_CONFIRM
                            commit
180ms                       ◄──── committed
        NOTIFICATION ◄──────┼─────────────────► NOTIFICATION
220ms   ACK ───────────────►│◄───────────────────────── ACK
        FINAL ◄─────────────┼──────────────────────► FINAL
250ms   (complete)                                 (complete)
```

---

## Appendix B: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-17 | Initial draft |
