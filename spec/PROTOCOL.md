# AtomicSettle Protocol Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-01-17

## Abstract

AtomicSettle is an open protocol for real-time cross-border settlement between financial institutions. This specification defines the core protocol, including network topology, settlement lifecycle, message flow, and failure handling.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Terminology](#2-terminology)
3. [Network Topology](#3-network-topology)
4. [Settlement Model](#4-settlement-model)
5. [Settlement Lifecycle](#5-settlement-lifecycle)
6. [Message Flow](#6-message-flow)
7. [Timing Requirements](#7-timing-requirements)
8. [Failure Handling](#8-failure-handling)
9. [Netting](#9-netting)
10. [FX Integration](#10-fx-integration)
11. [Compliance Hooks](#11-compliance-hooks)
12. [Versioning](#12-versioning)

---

## 1. Introduction

### 1.1 Purpose

This document specifies the AtomicSettle protocol, an open standard for enabling real-time, atomic settlement of cross-border payments between financial institutions.

### 1.2 Scope

This specification covers:
- Core protocol semantics and message flow
- Settlement lifecycle and state machine
- Failure modes and recovery procedures
- Timing and performance requirements

This specification does NOT cover:
- Implementation details (see reference implementation)
- Deployment architecture (see operations guide)
- SDK-specific behaviors (see SDK documentation)

### 1.3 Design Principles

1. **Atomicity**: A settlement either fully completes or fully rolls back. No partial states.
2. **Transparency**: All state transitions are auditable and deterministic.
3. **Finality**: Once confirmed, a settlement cannot be reversed at the protocol level.
4. **Interoperability**: Built on ISO 20022 concepts and message structures.
5. **Resilience**: The protocol handles failures gracefully with bounded recovery time.

### 1.4 Background

Cross-border payments today suffer from:
- Multi-day settlement times
- Opacity (lack of tracking)
- High costs (3-5% of transaction value)
- $27 trillion trapped in nostro accounts globally

AtomicSettle addresses these issues by providing a standardized protocol for instant settlement with full transparency.

---

## 2. Terminology

### 2.1 Core Terms

| Term | Definition |
|------|------------|
| **Coordinator** | A trusted entity that orchestrates settlement between participants. Provides atomicity guarantees. |
| **Participant** | A financial institution (bank, fintech, central bank) that can send or receive settlements. |
| **Settlement** | An atomic transfer of value between two or more participants. |
| **Leg** | A single directional transfer within a settlement. A settlement has one or more legs. |
| **Lock** | A temporary reservation of funds that prevents double-spending during settlement. |
| **Finality** | The irrevocable and unconditional completion of a settlement. |

### 2.2 Account Terms

| Term | Definition |
|------|------------|
| **Settlement Account** | An account held at the coordinator representing a participant's position. |
| **Nostro Account** | "Our money held by them" - A bank's account at another bank. |
| **Vostro Account** | "Their money held by us" - Another bank's account held at our bank. |
| **Position** | A participant's net balance across all currencies at the coordinator. |

### 2.3 State Terms

| Term | Definition |
|------|------------|
| **INITIATED** | Settlement request received and validated. |
| **LOCKED** | Funds locked at source participant(s). |
| **COMMITTED** | Atomic transfer executed by coordinator. |
| **SETTLED** | All participants acknowledged; settlement final. |
| **FAILED** | Settlement could not complete; rollback executed. |
| **REJECTED** | Settlement rejected during validation. |

---

## 3. Network Topology

### 3.1 Architecture Overview

```
                         ┌─────────────────────────────┐
                         │     Coordinator Cluster     │
                         │  ┌─────┐ ┌─────┐ ┌─────┐   │
                         │  │ C1  │ │ C2  │ │ C3  │   │
                         │  └──┬──┘ └──┬──┘ └──┬──┘   │
                         │     └───────┼───────┘       │
                         │         Raft Consensus      │
                         └─────────────┬───────────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
    ┌─────────┴─────────┐    ┌─────────┴─────────┐    ┌─────────┴─────────┐
    │   Participant A    │    │   Participant B    │    │   Participant C    │
    │  ┌─────────────┐   │    │  ┌─────────────┐   │    │  ┌─────────────┐   │
    │  │   Adapter   │   │    │  │   Adapter   │   │    │  │   Adapter   │   │
    │  └──────┬──────┘   │    │  └──────┬──────┘   │    │  └──────┬──────┘   │
    │         │          │    │         │          │    │         │          │
    │  ┌──────┴──────┐   │    │  ┌──────┴──────┐   │    │  ┌──────┴──────┐   │
    │  │ Core Banking│   │    │  │ Core Banking│   │    │  │ Core Banking│   │
    │  └─────────────┘   │    │  └─────────────┘   │    │  └─────────────┘   │
    └────────────────────┘    └────────────────────┘    └────────────────────┘
```

### 3.2 Coordinator Cluster

The coordinator operates as a fault-tolerant cluster using Raft consensus:

- **Minimum nodes**: 3 (tolerates 1 failure)
- **Recommended nodes**: 5 (tolerates 2 failures)
- **Maximum nodes**: 7 (beyond this, latency increases)

#### 3.2.1 Leader Election

- Uses Raft leader election
- Leader processes all settlement requests
- Followers replicate state and can take over if leader fails
- Election timeout: 150-300ms

#### 3.2.2 State Replication

All settlement state transitions are replicated before acknowledgment:

1. Leader receives settlement request
2. Leader appends to local log
3. Leader replicates to followers
4. On majority acknowledgment, leader commits
5. Leader responds to participant

### 3.3 Participant Connection

Participants connect to the coordinator via:

- **Protocol**: gRPC over HTTP/2
- **Authentication**: Mutual TLS (mTLS) with certificate pinning
- **Connection**: Persistent bidirectional stream
- **Heartbeat**: Every 5 seconds
- **Reconnection**: Exponential backoff (1s, 2s, 4s, 8s, max 30s)

### 3.4 Network Requirements

| Requirement | Specification |
|-------------|---------------|
| Latency (coordinator ↔ participant) | < 50ms RTT recommended |
| Bandwidth | 1 Mbps minimum per participant |
| Availability | Redundant network paths recommended |

---

## 4. Settlement Model

### 4.1 Settlement Types

#### 4.1.1 Simple Settlement (Two-Party)

Transfer between two participants in a single currency:

```
Participant A ──[$100 USD]──► Participant B
```

#### 4.1.2 FX Settlement (Two-Party, Two-Currency)

Transfer with foreign exchange:

```
Participant A ──[$100 USD]──► Coordinator ──[€92 EUR]──► Participant B
```

#### 4.1.3 Multi-Leg Settlement

Transfer involving multiple participants:

```
Participant A ──[$50 USD]──► Participant B
       │
       └──────[$50 USD]──► Participant C
```

### 4.2 Atomicity Guarantee

All legs of a settlement complete together or none complete:

- If Leg 1 succeeds and Leg 2 fails → Leg 1 is rolled back
- Participants never see partial state
- Coordinator guarantees atomic commit across all legs

### 4.3 Settlement Finality

Once a settlement reaches `SETTLED` state:

1. It cannot be reversed at the protocol level
2. All participant ledgers reflect the final state
3. Audit records are immutable

**Note**: Finality at the protocol level does not prevent business-level reversals (e.g., a separate refund settlement).

---

## 5. Settlement Lifecycle

### 5.1 State Machine

```
                         ┌──────────────────────────────────────┐
                         │                                      │
                         ▼                                      │
    ┌─────────┐     ┌─────────┐     ┌────────┐     ┌─────────┐ │
    │INITIATED├────►│VALIDATED├────►│ LOCKED ├────►│COMMITTED├─┤
    └────┬────┘     └────┬────┘     └───┬────┘     └────┬────┘ │
         │               │              │               │      │
         │               │              │               │      │
         ▼               ▼              ▼               ▼      │
    ┌─────────┐     ┌─────────┐     ┌────────┐     ┌─────────┐ │
    │REJECTED │     │REJECTED │     │ FAILED │     │ SETTLED │◄┘
    └─────────┘     └─────────┘     └────────┘     └─────────┘
```

### 5.2 State Definitions

#### 5.2.1 INITIATED

- Settlement request received by coordinator
- Request ID assigned
- Basic format validation complete
- **Next**: VALIDATED or REJECTED

#### 5.2.2 VALIDATED

- All participants verified and active
- Compliance checks passed
- Credit limits verified
- FX rate locked (if applicable)
- **Next**: LOCKED or REJECTED

#### 5.2.3 LOCKED

- Funds locked at source participant(s)
- Lock has expiration time (default: 30 seconds)
- **Next**: COMMITTED or FAILED

#### 5.2.4 COMMITTED

- Atomic transfer executed on coordinator ledger
- Cannot be rolled back
- **Next**: SETTLED

#### 5.2.5 SETTLED

- All participants acknowledged
- Final state
- Audit records complete

#### 5.2.6 REJECTED

- Settlement could not be initiated
- Reasons: Invalid participant, compliance failure, insufficient credit
- No funds were locked

#### 5.2.7 FAILED

- Settlement could not complete after locking
- Funds have been unlocked (rolled back)
- Reasons: Lock timeout, participant unavailable, system error

### 5.3 State Transition Rules

| From | To | Condition |
|------|-----|-----------|
| INITIATED | VALIDATED | All validation checks pass |
| INITIATED | REJECTED | Any validation check fails |
| VALIDATED | LOCKED | All source locks acquired |
| VALIDATED | REJECTED | Lock acquisition fails immediately |
| LOCKED | COMMITTED | Coordinator commits transfer |
| LOCKED | FAILED | Lock timeout or participant failure |
| COMMITTED | SETTLED | All participants acknowledge |

---

## 6. Message Flow

### 6.1 Successful Settlement Flow

```
Participant A          Coordinator          Participant B
     │                      │                      │
     │ SETTLE_REQUEST       │                      │
     │─────────────────────►│                      │
     │                      │                      │
     │                      │ (validate)           │
     │                      │                      │
     │ SETTLE_LOCK          │                      │
     │◄─────────────────────│                      │
     │                      │                      │
     │ LOCK_CONFIRM         │                      │
     │─────────────────────►│                      │
     │                      │                      │
     │                      │ (commit)             │
     │                      │                      │
     │ SETTLE_COMMITTED     │ SETTLE_COMMITTED     │
     │◄─────────────────────│─────────────────────►│
     │                      │                      │
     │ SETTLE_ACK           │ SETTLE_ACK           │
     │─────────────────────►│◄─────────────────────│
     │                      │                      │
     │ SETTLE_FINAL         │ SETTLE_FINAL         │
     │◄─────────────────────│─────────────────────►│
     │                      │                      │
```

### 6.2 Failed Settlement Flow (Lock Timeout)

```
Participant A          Coordinator          Participant B
     │                      │                      │
     │ SETTLE_REQUEST       │                      │
     │─────────────────────►│                      │
     │                      │                      │
     │ SETTLE_LOCK          │                      │
     │◄─────────────────────│                      │
     │                      │                      │
     │ (no response - timeout)                     │
     │                      │                      │
     │ SETTLE_ABORT         │                      │
     │◄─────────────────────│                      │
     │                      │                      │
```

### 6.3 Message Types Summary

| Message | Direction | Purpose |
|---------|-----------|---------|
| SETTLE_REQUEST | Participant → Coordinator | Initiate settlement |
| SETTLE_VALIDATE | Coordinator → Participant | Request validation (optional) |
| SETTLE_LOCK | Coordinator → Participant | Request fund lock |
| LOCK_CONFIRM | Participant → Coordinator | Confirm lock acquired |
| SETTLE_COMMITTED | Coordinator → Participant | Notify commit complete |
| SETTLE_ACK | Participant → Coordinator | Acknowledge receipt |
| SETTLE_FINAL | Coordinator → Participant | Confirm finality |
| SETTLE_ABORT | Coordinator → Participant | Rollback settlement |

---

## 7. Timing Requirements

### 7.1 Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| End-to-end latency (p50) | < 1000ms | From request to final |
| End-to-end latency (p99) | < 3000ms | Including retries |
| Lock acquisition | < 500ms | Coordinator to participant |
| Commit execution | < 200ms | Coordinator internal |
| Throughput | 10,000 TPS | Sustained, single coordinator cluster |

### 7.2 Timeouts

| Operation | Timeout | Recovery |
|-----------|---------|----------|
| Lock acquisition | 10s | Abort settlement |
| Lock hold (max) | 30s | Auto-release, abort settlement |
| Participant acknowledgment | 60s | Log warning, continue (fire-and-forget) |
| Coordinator heartbeat | 15s | Mark participant offline |

### 7.3 Clock Synchronization

- All nodes MUST use NTP or similar
- Maximum clock skew: 100ms
- Lock expiration includes skew tolerance

---

## 8. Failure Handling

### 8.1 Failure Categories

#### 8.1.1 Participant Failures

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Network disconnect | Heartbeat timeout | Reconnect, resume |
| Crash during lock | No LOCK_CONFIRM | Abort settlement |
| Crash after commit | No SETTLE_ACK | Idempotent retry on reconnect |

#### 8.1.2 Coordinator Failures

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Leader failure | Heartbeat timeout | Raft election, new leader |
| Network partition | Split-brain detection | Minority partition stops accepting writes |
| Data corruption | Checksum failure | Restore from replicas |

### 8.2 Recovery Procedures

#### 8.2.1 Participant Reconnection

When a participant reconnects after disconnection:

1. Authenticate with coordinator
2. Receive list of pending settlements
3. For each pending settlement:
   - If LOCKED: Confirm lock still held or report release
   - If COMMITTED: Send SETTLE_ACK
4. Resume normal operation

#### 8.2.2 Coordinator Recovery

When a new coordinator leader is elected:

1. Load committed state from Raft log
2. For settlements in LOCKED state:
   - If lock expired: Transition to FAILED
   - If lock valid: Await LOCK_CONFIRM or timeout
3. For settlements in COMMITTED state:
   - Retry SETTLE_COMMITTED to participants
4. Resume normal operation

### 8.3 Idempotency

All messages are idempotent. Duplicate handling:

- SETTLE_REQUEST with same `idempotency_key`: Return existing result
- LOCK_CONFIRM for already-confirmed lock: Acknowledge
- SETTLE_ACK for already-acked settlement: Acknowledge

---

## 9. Netting

### 9.1 Overview

Netting reduces gross settlement obligations by offsetting mutual positions. AtomicSettle supports optional multilateral netting.

### 9.2 Netting Window

- Default window: 100ms (configurable)
- Settlements within window are candidates for netting
- Netting is best-effort; non-netted settlements proceed normally

### 9.3 Netting Example

Without netting:
```
Bank A → Bank B: $100
Bank B → Bank A: $80
Gross flow: $180
```

With netting:
```
Bank A → Bank B: $20 (net)
Gross flow: $20
```

### 9.4 Netting Calculation

Coordinator calculates net positions at end of netting window:

```
net_position[A][B] = sum(A→B settlements) - sum(B→A settlements)
```

Only net amounts are settled on the ledger.

---

## 10. FX Integration

### 10.1 FX Rate Handling

Three modes for FX settlements:

#### 10.1.1 AT_SOURCE

- Sender provides converted amount
- Sender bears FX risk
- Coordinator validates rate is reasonable

#### 10.1.2 AT_DESTINATION

- Sender provides source amount
- Receiver performs conversion
- Receiver bears FX risk

#### 10.1.3 AT_COORDINATOR (Recommended)

- Coordinator performs conversion
- Uses locked rate at validation time
- Rate lock valid for 30 seconds

### 10.2 Rate Sources

Coordinator aggregates rates from multiple sources:

1. Primary rate providers (Reuters, Bloomberg)
2. Participant-provided rates
3. Central bank reference rates

Final rate = median of available sources (Byzantine fault tolerant).

### 10.3 Rate Lock

When FX settlement is validated:

1. Coordinator locks current rate
2. Rate valid for `rate_lock_duration` (default: 30s)
3. If settlement completes within window, locked rate is used
4. If lock expires, settlement fails (can retry with new rate)

---

## 11. Compliance Hooks

### 11.1 Overview

AtomicSettle provides hooks for compliance checks without mandating specific implementations.

### 11.2 Hook Points

| Hook | Trigger | Can Block |
|------|---------|-----------|
| PRE_VALIDATE | Before validation | Yes |
| POST_VALIDATE | After validation | Yes |
| PRE_LOCK | Before lock request | Yes |
| POST_COMMIT | After commit | No |
| POST_SETTLE | After final | No |

### 11.3 Hook Interface

```
interface ComplianceHook {
    fn check(settlement: &Settlement, context: &Context) -> HookResult;
}

enum HookResult {
    Approve,
    Reject(reason: String),
    RequestManualReview(reason: String),
}
```

### 11.4 Standard Compliance Checks

Recommended checks (not protocol-mandated):

- Sanctions screening (OFAC, EU, UN)
- AML pattern detection
- Transaction limits
- Jurisdiction restrictions

---

## 12. Versioning

### 12.1 Protocol Version

Protocol version follows semantic versioning: `MAJOR.MINOR.PATCH`

- MAJOR: Breaking changes
- MINOR: New features, backward compatible
- PATCH: Bug fixes

### 12.2 Version Negotiation

On connection, participant and coordinator exchange versions:

```
CONNECT(supported_versions: [1.0, 1.1, 0.9])
CONNECTED(selected_version: 1.1)
```

### 12.3 Backward Compatibility

- Coordinator MUST support current version and previous MAJOR version
- Participants SHOULD upgrade within 12 months of new MAJOR version
- Deprecation notice: 6 months before dropping version support

---

## Appendix A: References

1. CLS Bank - Continuous Linked Settlement
2. SWIFT gpi - Global Payments Innovation
3. BIS - Project Nexus
4. ISO 20022 - Financial Services Messaging
5. CPMI-IOSCO - Principles for Financial Market Infrastructures
6. Raft Consensus Algorithm

## Appendix B: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-17 | Initial draft |
