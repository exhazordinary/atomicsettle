# Core Concepts

Understanding the key concepts behind AtomicSettle.

## Overview

AtomicSettle is an open protocol for real-time cross-border settlement between financial institutions. It enables instant, atomic settlement of payments between banks, similar to how CLS Bank settles $6 trillion daily in foreign exchange.

## Key Terms

### Coordinator

The **coordinator** is a trusted entity that orchestrates settlements between participants. It:

- Validates settlement requests
- Manages distributed locks
- Executes atomic commits
- Ensures settlement finality

In production, coordinators are typically operated by:
- Central banks
- Banking consortiums
- Trusted third parties

### Participant

A **participant** is a financial institution (bank, fintech, or other licensed entity) that can send or receive settlements. Each participant:

- Connects to one or more coordinators
- Maintains settlement accounts at the coordinator
- Signs messages with cryptographic keys
- Processes incoming settlement notifications

### Settlement

A **settlement** is an atomic transfer of value between participants. Settlements can:

- Be single-currency or cross-currency
- Involve two or more participants
- Include multiple legs (e.g., payment-vs-payment)

### Settlement Leg

A **leg** is a single directional transfer within a settlement. A simple settlement has one leg; complex settlements (like FX) may have multiple legs that execute atomically.

### Lock

A **lock** is a temporary reservation of funds at a participant. Locks:

- Prevent double-spending during settlement
- Have a time limit (typically 30 seconds)
- Are released on success or failure

### Finality

**Finality** means a settlement cannot be reversed at the protocol level. Once a settlement reaches the `SETTLED` state:

- All ledgers reflect the final state
- No protocol-level reversal is possible
- Business-level reversals require a new settlement

## Settlement Lifecycle

```
INITIATED → VALIDATED → LOCKED → COMMITTED → SETTLED
     │           │          │         │
     └─► REJECTED ◄─────────┴─────────┘
                          FAILED
```

### States

| State | Description |
|-------|-------------|
| INITIATED | Request received, format validated |
| VALIDATED | All checks passed, ready for locking |
| LOCKED | Funds locked at source participant(s) |
| COMMITTED | Atomic transfer executed on coordinator ledger |
| SETTLED | Complete, all parties acknowledged |
| REJECTED | Could not process (validation failed) |
| FAILED | Failed after partial processing (locks released) |

## Network Topology

```
                    ┌─────────────────┐
                    │   Coordinator   │
                    │     Cluster     │
                    └────────┬────────┘
           ┌─────────────────┼─────────────────┐
           │                 │                 │
    ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐
    │ Participant │   │ Participant │   │ Participant │
    │   Bank A    │   │   Bank B    │   │   Bank C    │
    └─────────────┘   └─────────────┘   └─────────────┘
```

### Why a Coordinator?

Fully decentralized atomic settlement without a trusted party is an unsolved computer science problem. The coordinator model:

- Is proven at scale (CLS Bank)
- Provides strong atomicity guarantees
- Enables regulatory compliance
- Supports multiple trust models

## Accounts and Balances

### Nostro/Vostro Accounts

Traditional correspondent banking uses nostro/vostro accounts:

- **Nostro**: "Our money held by them"
- **Vostro**: "Their money held by us"

AtomicSettle uses **settlement accounts** at the coordinator, eliminating the need for bilateral nostro accounts.

### Balance Types

| Type | Description |
|------|-------------|
| Available | Can be used for settlements |
| Locked | Reserved for pending settlements |
| Pending In | Incoming (not yet final) |
| Pending Out | Outgoing (not yet final) |

## FX Handling

AtomicSettle supports three FX modes:

### AT_SOURCE

Sender converts before sending. Sender bears FX risk.

### AT_DESTINATION

Receiver converts after receiving. Receiver bears FX risk.

### AT_COORDINATOR (Recommended)

Coordinator performs conversion using locked rate. Provides rate certainty during settlement.

## Security

### Authentication

- Mutual TLS (mTLS) with X.509 certificates
- Ed25519 message signatures
- Certificate-based participant identity

### Encryption

- TLS 1.3 for transport
- AES-256-GCM for message encryption
- X25519 for key agreement

### Audit

- All state transitions logged
- Cryptographic audit trail
- Tamper-evident logging

## Next Steps

- [Architecture](../architecture/) - Detailed system design
- [Protocol Specification](../../spec/PROTOCOL.md) - Full protocol details
- [Security Model](../../spec/SECURITY.md) - Security in depth
