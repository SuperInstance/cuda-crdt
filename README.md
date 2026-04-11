# cuda-crdt

CRDTs — G-Counter, PN-Counter, OR-Set, LWW-Register, Vector Clock (Rust)

Part of the Cocapn fleet — a Lucineer vessel component.

## What It Does

### Key Types

- `VectorClock` — core data structure
- `GCounter` — core data structure
- `PNCounter` — core data structure
- `GSet<T: Clone + Eq + std::hash::Hash + Serialize>` — core data structure
- `LWWRegister<T: Clone + Serialize>` — core data structure
- `ORSet<T: Clone + Eq + std::hash::Hash + Serialize>` — core data structure
- _and 1 more (see source)_

## Quick Start

```bash
# Clone
git clone https://github.com/Lucineer/cuda-crdt.git
cd cuda-crdt

# Build
cargo build

# Run tests
cargo test
```

## Usage

```rust
use cuda_crdt::*;

// See src/lib.rs for full API
// 11 unit tests included
```

### Available Implementations

- `VectorClock` — see source for methods
- `GCounter` — see source for methods
- `PNCounter` — see source for methods

## Testing

```bash
cargo test
```

11 unit tests covering core functionality.

## Architecture

This crate is part of the **Cocapn Fleet** — a git-native multi-agent ecosystem.

- **Category**: other
- **Language**: Rust
- **Dependencies**: See `Cargo.toml`
- **Status**: Active development

## Related Crates


## Fleet Position

```
Casey (Captain)
├── JetsonClaw1 (Lucineer realm — hardware, low-level systems, fleet infrastructure)
├── Oracle1 (SuperInstance — lighthouse, architecture, consensus)
└── Babel (SuperInstance — multilingual scout)
```

## Contributing

This is a fleet vessel component. Fork it, improve it, push a bottle to `message-in-a-bottle/for-jetsonclaw1/`.

## License

MIT

---

*Built by JetsonClaw1 — part of the Cocapn fleet*
*See [cocapn-fleet-readme](https://github.com/Lucineer/cocapn-fleet-readme) for the full fleet roadmap*
