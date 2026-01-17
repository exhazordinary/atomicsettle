# Architecture Overview

AtomicSettle's architecture is designed for high availability, low latency, and regulatory compliance.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          COORDINATOR CLUSTER                            │
│                                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                 │
│  │   Node 1    │    │   Node 2    │    │   Node 3    │                 │
│  │  (Leader)   │◄──►│ (Follower)  │◄──►│ (Follower)  │                 │
│  └──────┬──────┘    └─────────────┘    └─────────────┘                 │
│         │                Raft Consensus                                 │
│  ┌──────┴────────────────────────────────────────────────────────────┐ │
│  │                      Shared Services                               │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐          │ │
│  │  │Settlement│  │   Lock   │  │  Ledger  │  │    FX    │          │ │
│  │  │Processor │  │ Manager  │  │  Engine  │  │  Engine  │          │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘          │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                        Data Layer                                  │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            │ │
│  │  │  PostgreSQL  │  │    Redis     │  │    NATS      │            │ │
│  │  │   (Ledger)   │  │   (Cache)    │  │  (Messaging) │            │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────┬─────────────────────────────────────┘
                                    │
                            gRPC / mTLS
                                    │
    ┌───────────────────────────────┼───────────────────────────────┐
    │                               │                               │
┌───┴───────────┐           ┌───────┴───────┐           ┌───────────┴───┐
│  Participant  │           │  Participant  │           │  Participant  │
│    Bank A     │           │    Bank B     │           │    Bank C     │
│               │           │               │           │               │
│ ┌───────────┐ │           │ ┌───────────┐ │           │ ┌───────────┐ │
│ │   SDK     │ │           │ │   SDK     │ │           │ │   SDK     │ │
│ └─────┬─────┘ │           │ └─────┬─────┘ │           │ └─────┬─────┘ │
│       │       │           │       │       │           │       │       │
│ ┌─────┴─────┐ │           │ ┌─────┴─────┐ │           │ ┌─────┴─────┐ │
│ │Core Bank  │ │           │ │Core Bank  │ │           │ │Core Bank  │ │
│ │  System   │ │           │ │  System   │ │           │ │  System   │ │
│ └───────────┘ │           │ └───────────┘ │           │ └───────────┘ │
└───────────────┘           └───────────────┘           └───────────────┘
```

## Component Overview

### Coordinator Cluster

The coordinator runs as a fault-tolerant cluster using Raft consensus:

| Component | Purpose |
|-----------|---------|
| Settlement Processor | Orchestrates settlement lifecycle |
| Lock Manager | Manages distributed locks |
| Ledger Engine | Double-entry bookkeeping |
| FX Engine | Currency conversion |

### Data Layer

| Component | Purpose | Characteristics |
|-----------|---------|-----------------|
| PostgreSQL | Ledger storage | ACID, durable, point-in-time recovery |
| Redis | Caching, locks | Low latency, distributed locks |
| NATS | Messaging | High throughput, persistence optional |

### Participant Integration

Banks integrate via SDK:

| SDK | Language | Use Case |
|-----|----------|----------|
| Python | Python 3.9+ | Prototyping, scripts |
| Java | Java 11+ | Enterprise systems |
| TypeScript | Node.js 18+ | Modern fintech |
| Go | Go 1.21+ | Infrastructure |

## Data Flow

### Settlement Flow

```
1. Participant A sends SettleRequest
   │
   ▼
2. Coordinator validates request
   - Check participants exist
   - Verify compliance
   - Lock FX rate (if applicable)
   │
   ▼
3. Coordinator acquires locks
   - Send LockRequest to Participant A
   - Wait for LockConfirm
   │
   ▼
4. Coordinator commits (atomic)
   - Begin DB transaction
   - Debit source account
   - Credit destination account
   - Mark locks consumed
   - Commit transaction
   │
   ▼
5. Notify participants
   - Send SettlementNotification to A and B
   - Wait for acknowledgments
   │
   ▼
6. Mark settled
```

### Message Flow

```
Participant A          Coordinator          Participant B
     │                      │                      │
     │ SETTLE_REQUEST       │                      │
     │─────────────────────►│                      │
     │                      │                      │
     │ SETTLE_LOCK          │                      │
     │◄─────────────────────│                      │
     │                      │                      │
     │ LOCK_CONFIRM         │                      │
     │─────────────────────►│                      │
     │                      │                      │
     │ SETTLE_COMMITTED     │ SETTLE_COMMITTED     │
     │◄─────────────────────│─────────────────────►│
     │                      │                      │
```

## Scalability

### Horizontal Scaling

| Component | Scaling Strategy |
|-----------|------------------|
| Coordinator | Add nodes (3, 5, 7) |
| PostgreSQL | Read replicas, sharding |
| Redis | Cluster mode |
| NATS | Cluster with routes |

### Performance Targets

| Metric | Target |
|--------|--------|
| Latency (p99) | < 3 seconds |
| Throughput | 10,000 TPS |
| Availability | 99.99% |

## High Availability

### Coordinator HA

- Raft consensus (tolerates N/2 failures)
- Automatic leader election
- State replicated before acknowledgment

### Data HA

- PostgreSQL: Streaming replication
- Redis: Sentinel or Cluster
- NATS: Built-in clustering

### Network HA

- Multiple coordinator endpoints
- Client-side load balancing
- Automatic reconnection

## Security Architecture

See [Security Specification](../../spec/SECURITY.md) for details.

### Defense in Depth

```
┌─────────────────────────────────────────────────────┐
│                 Network Security                     │
│  - Firewall rules                                   │
│  - DDoS protection                                  │
│  ┌───────────────────────────────────────────────┐  │
│  │              Transport Security                │  │
│  │  - TLS 1.3                                    │  │
│  │  - mTLS authentication                        │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │          Message Security               │  │  │
│  │  │  - Ed25519 signatures                   │  │  │
│  │  │  - Message encryption                   │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Deployment Models

### Cloud Deployment

```yaml
# Kubernetes deployment example
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: coordinator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: coordinator
  template:
    spec:
      containers:
      - name: coordinator
        image: atomicsettle/coordinator:latest
        ports:
        - containerPort: 8080
```

### On-Premise Deployment

For banks with strict data residency requirements:

- Deploy coordinator in bank's data center
- Use HSM for key management
- Integrate with existing monitoring

## Next Steps

- [Settlement Flow](settlement-flow.md) - Detailed settlement process
- [Integration Guide](../integration-guide/) - How to integrate
- [Operations Guide](../operations/) - Running in production
