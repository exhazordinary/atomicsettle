# AtomicSettle Simulator

A test environment for banks and developers to test AtomicSettle integration without real money.

## Features

- **Simulated Banks**: Spin up multiple simulated bank nodes
- **Scenario Testing**: Run predefined test scenarios
- **Real-time Visualization**: Web dashboard showing settlement flows
- **Load Testing**: Performance testing at scale
- **Fault Injection**: Test failure scenarios

## Quick Start

```bash
# Start simulator with 3 banks
cargo run --bin simulator -- --banks 3

# Run a specific scenario
cargo run --bin simulator -- --scenario simple-settlement

# Start with web visualizer
cargo run --bin simulator -- --banks 5 --visualizer
```

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                   SIMULATION CONTROL PLANE                  │
│  - Scenario management                                      │
│  - Time manipulation (speed up/slow down)                   │
│  - Fault injection                                          │
│  - Metrics collection                                       │
└─────────────────────────────┬──────────────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────────┐
    │                         │                         │
┌───┴───┐               ┌─────┴─────┐             ┌─────┴─────┐
│ Sim   │               │   Real    │             │   Sim     │
│ Bank  │◄─────────────►│Coordinator│◄───────────►│   Bank    │
│  A    │               │           │             │    B      │
└───────┘               └───────────┘             └───────────┘
```

## Scenarios

### simple-settlement

Basic 2-party settlement in a single currency.

### multi-currency

Settlement with FX conversion through coordinator.

### multi-party

Settlement involving 3+ participants.

### high-volume

Stress test with thousands of concurrent settlements.

### failure-recovery

Test lock timeouts and participant failures.

## Web Visualizer

Access the real-time visualization dashboard at `http://localhost:8888` when running with `--visualizer`.

Features:
- Global map with bank locations
- Live settlement flows (animated)
- Latency histograms
- Success/failure rates
- Queue depths
