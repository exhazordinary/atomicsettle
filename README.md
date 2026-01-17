# AtomicSettle

**Open-Source Cross-Border Settlement Protocol**

AtomicSettle is an open protocol specification and reference implementation for real-time cross-border settlement between financial institutions. Our goal is to become the de-facto open standard for instant settlement, similar to how TCP/IP standardized internet communication.

## Vision

Build an open-source protocol that:
- Enables real-time cross-border settlement (< 3 seconds)
- Is transparent, auditable, and trusted by regulators
- Can be adopted by banks, fintechs, and central banks
- Reduces the $27 trillion trapped in nostro accounts globally

## Project Structure

```
atomicsettle/
├── spec/                    # Protocol specification (the RFC)
│   ├── PROTOCOL.md          # Core protocol specification
│   ├── MESSAGES.md          # Message format definitions
│   ├── SETTLEMENT.md        # Settlement lifecycle
│   ├── SECURITY.md          # Security requirements
│   └── COMPLIANCE.md        # Regulatory considerations
│
├── reference/               # Reference implementation (Rust)
│   ├── coordinator/         # Settlement coordinator node
│   ├── participant/         # Bank participant library
│   ├── ledger/              # Double-entry ledger engine
│   ├── fx/                  # FX rate engine
│   └── crypto/              # Cryptographic primitives
│
├── sdk/                     # Integration SDKs
│   ├── python/              # Python SDK for banks
│   ├── java/                # Java SDK (enterprise)
│   ├── typescript/          # TypeScript SDK
│   └── go/                  # Go SDK
│
├── simulator/               # Test environment
│   ├── banks/               # Simulated bank nodes
│   ├── scenarios/           # Test scenarios
│   ├── visualizer/          # Real-time visualization
│   └── load-tests/          # Performance testing
│
├── docs/                    # Documentation
└── examples/                # Example implementations
```

## Quick Start

### Running a Local Test Network

```bash
# Start coordinator cluster
cargo run --bin coordinator -- --config config/local.toml

# Start simulated banks
cargo run --bin simulator -- --banks 3 --scenario simple

# Send a test settlement
cargo run --example simple_settlement
```

### Using the Python SDK

```python
from atomicsettle import AtomicSettleClient, Currency
from decimal import Decimal

client = AtomicSettleClient(
    participant_id="BANK_A",
    coordinator_url="https://coordinator.atomicsettle.local",
    signing_key=load_key("private_key.pem")
)

settlement = await client.send(
    to_participant="BANK_B",
    amount=Decimal("1000000"),
    currency=Currency.USD,
    purpose="Trade settlement"
)

print(f"Settled in {settlement.duration_ms}ms")
```

## Architecture

AtomicSettle uses a coordinator-based architecture, similar to CLS Bank which settles $6 trillion daily in FX. This is a deliberate design choice:

```
                    ┌─────────────────┐
                    │   Coordinator   │
                    │     Network     │
                    └────────┬────────┘
           ┌─────────────────┼─────────────────┐
           │                 │                 │
    ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐
    │ Participant │   │ Participant │   │ Participant │
    │   Bank A    │   │   Bank B    │   │   Bank C    │
    └─────────────┘   └─────────────┘   └─────────────┘
```

**Why a Coordinator?**
- Fully decentralized atomic settlement across independent ledgers is an unsolved computer science problem
- CLS Bank has proven this model works at massive scale
- The coordinator can be operated by a central bank, consortium, or trusted third party
- The protocol is open—anyone can run a coordinator network

## Performance Targets

| Metric | Target |
|--------|--------|
| Settlement latency (p99) | < 3 seconds |
| Availability | 99.99% |
| Throughput | 10,000 settlements/second |

## Regulatory Alignment

AtomicSettle is designed to comply with:
- FSB's 2027 cross-border payments roadmap
- ISO 20022 messaging standards
- CPMI-IOSCO Principles for Financial Market Infrastructures
- FATF AML/CFT recommendations

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/atomicsettle/atomicsettle.git
cd atomicsettle

# Install Rust toolchain
rustup default stable

# Build all components
cargo build --workspace

# Run tests
cargo test --workspace

# Run lints
cargo clippy --workspace
```

## License

Apache 2.0 - See [LICENSE](LICENSE) for details.

## Acknowledgments

AtomicSettle builds on decades of payment systems research and the work of organizations like:
- CLS Bank
- SWIFT
- Bank for International Settlements (BIS)
- Committee on Payments and Market Infrastructures (CPMI)
