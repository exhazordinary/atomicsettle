# Quick Start Guide

Get started with AtomicSettle in 5 minutes.

## Prerequisites

- Rust 1.75+ (for running coordinator)
- Python 3.9+ (for SDK)
- Docker (optional, for containerized deployment)

## Installation

### Using Pre-built Binaries

```bash
# Download the latest release
curl -L https://github.com/atomicsettle/atomicsettle/releases/latest/download/atomicsettle-linux-amd64.tar.gz | tar xz

# Move to PATH
sudo mv atomicsettle-coordinator /usr/local/bin/
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/atomicsettle/atomicsettle.git
cd atomicsettle

# Build
cd reference
cargo build --release

# Binary will be at target/release/coordinator
```

### Python SDK

```bash
pip install atomicsettle
```

## Running the Demo

### 1. Start the Coordinator

```bash
# Start with default configuration
atomicsettle-coordinator

# Or with custom config
atomicsettle-coordinator --config config.toml
```

### 2. Run the Simulator

In a new terminal:

```bash
# Start simulator with 3 banks
cd simulator
cargo run -- --banks 3 --scenario simple-settlement
```

### 3. Send a Settlement (Python)

```python
import asyncio
from decimal import Decimal
from atomicsettle import AtomicSettleClient, Currency

async def main():
    client = AtomicSettleClient(
        participant_id="BANK_A",
        coordinator_url="http://localhost:8080"
    )

    await client.connect()

    settlement = await client.send(
        to_participant="BANK_B",
        amount=Decimal("1000"),
        currency=Currency.USD,
        purpose="Test settlement"
    )

    print(f"Settlement {settlement.id}: {settlement.status}")
    print(f"Duration: {settlement.duration_ms}ms")

    await client.disconnect()

asyncio.run(main())
```

## Next Steps

- [Core Concepts](concepts.md) - Understand the protocol
- [Architecture](../architecture/) - System design
- [Integration Guide](../integration-guide/) - Full integration walkthrough
- [API Reference](../api-reference/) - Complete API documentation
