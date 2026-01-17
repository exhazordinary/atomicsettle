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

### Prerequisites

- Rust toolchain (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Running the Simulator

The simulator creates a test environment with simulated banks that execute settlements.

```bash
cd simulator

# Run with 3 simulated banks for 10 seconds
cargo run --bin simulator -- --banks 3 --duration 10

# Run with 5 banks continuously (Ctrl+C to stop)
cargo run --bin simulator -- --banks 5

# Run with faster simulation (2x speed)
cargo run --bin simulator -- --banks 3 --speed 2.0

# Run with reproducible results
cargo run --bin simulator -- --banks 3 --seed 12345
```

**Example output:**
```
INFO simulator: Starting AtomicSettle Simulator
INFO simulator: Banks: 3
INFO simulator::controller: Initialized bank BANK_A with $100000000 balance
INFO simulator::controller: Initialized bank BANK_B with $100000000 balance
INFO simulator::controller: Initialized bank BANK_C with $100000000 balance
INFO simulator::controller: Generating settlement: BANK_C -> BANK_B for $772711
INFO simulator::controller: Generating settlement: BANK_A -> BANK_C for $493063
INFO simulator: Simulation complete
INFO simulator: Total settlements: 5
INFO simulator: Successful: 5
INFO simulator: Average latency: 301ms
```

### Simulator CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `-b, --banks <N>` | Number of simulated banks | 3 |
| `-s, --scenario <NAME>` | Scenario to run (simple-settlement, multi-currency, etc.) | none |
| `--duration <SECS>` | Run duration in seconds (0 = infinite) | 0 |
| `--speed <MULT>` | Simulation speed multiplier | 1.0 |
| `--seed <NUM>` | Random seed for reproducibility | random |
| `--visualizer` | Enable web dashboard | false |
| `--visualizer-port <PORT>` | Web dashboard port | 8888 |

### Running Tests

```bash
# Run all tests in reference implementation
cd reference
cargo test

# Run simulator tests
cd simulator
cargo test

# Run tests for a specific crate
cargo test -p atomicsettle-crypto
cargo test -p atomicsettle-fx
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
