# AGENTS.md - Development Guide for Kameo

This guide provides essential information for agentic coding systems working in the Kameo repository.

## Project Overview

Kameo is a high-performance, lightweight Rust library for building fault-tolerant, asynchronous actor-based systems on Tokio. Workspace with 3 crates: `kameo` (main), `kameo_actors` (utility actors), `kameo_macros` (derive macros). Rust Edition 2024, minimum Rust 1.88.0 (main), 1.85.1 (macros).

---

## Build, Test, and Lint Commands

### Build
```bash
cargo build                    # Build all workspace members
cargo build --release          # Release build with LTO
cargo build -p kameo           # Build specific crate
cargo check                    # Quick compile check
```

### Test
```bash
cargo test                     # Run all tests
cargo test -p kameo            # Test specific crate
cargo test test_name           # Run specific test by name
cargo test module_name::test   # Run test in specific module
cargo test --lib               # Run library tests only
cargo test --features remote   # Run tests with specific feature
```

### Format & Lint
```bash
cargo fmt / cargo fmt --check  # Format/check formatting
cargo clippy / cargo clippy --fix  # Lint/auto-fix
```

---

## Code Style Guidelines

### Formatting
- **Always run `cargo fmt` before committing** - uses standard Rust formatting
- Imports grouped: `std` → external crates → internal crate modules

### Import Style
```rust
use std::{sync::Arc, time::Duration};  // Standard library
use futures::Future;                    // External crates
use crate::{Actor, ActorRef};          // Internal crate
```

**Rules:**
- Use `use crate::{multi, items}` for 2+ internal imports
- Re-export public API items at crate root (`pub use actor::Actor`)

### Type & Naming Conventions
```rust
// Types/Traits: PascalCase
pub struct MyActor { count: i64 }
pub trait Message<T: Send + 'static> { ... }

// Functions/Methods: snake_case
pub fn spawn() -> ActorRef<Self> { ... }
pub async fn handle(&mut self) -> Self::Reply { ... }

// Constants: SCREAMING_SNAKE_CASE
const DEFAULT_MAILBOX_CAPACITY: usize = 64;

// Type aliases: PascalCase
pub type BoxMessage<A> = Box<dyn DynMessage<A>>;
```

### Error Handling
```rust
// Actor errors via Actor::Error associated type
impl Actor for MyActor {
    type Error = Infallible;  // Use Infallible if never errors
}

// Return Result for fallible operations
pub fn from_bytes(bytes: &[u8]) -> Result<Self, ActorIdFromBytesError> { ... }

// Errors implement std::error::Error, Display, Debug
```

### Documentation
- **All public items MUST have `///` Rustdoc comments** (enforced by `#![warn(missing_docs)]`)
- Include usage examples using ````rust,ignore` blocks

### Async/Await Patterns
```rust
// Async functions return `impl Future<Output = T> + Send` or `BoxFuture<T>`
async fn handle(&mut self, msg: M) -> Self::Reply { ... }

// Use `#[tokio::main]` or `#[tokio::test]` attributes
```

### Attribute Lints (lib.rs defaults)
```rust
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
#![warn(missing_debug_implementations)]
#![deny(unused_must_use)]
```

Override with `#[allow(...)]` only when justified.

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_id_equality() { ... }

    #[tokio::test]
    async fn test_async_message() { ... }
}
```

Examples in `examples/` serve as integration tests.

### Actor-Specific Patterns
```rust
#[derive(Actor, Default)]
pub struct MyActor { count: i64 }

impl Message<Inc> for MyActor {
    type Reply = i64;

    async fn handle(&mut self, msg: Inc, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.count += msg.amount;
        self.count
    }
}

use kameo::prelude::*;  // Common imports
```

### Feature Flags
- `default = ["macros", "tracing"]`
- `macros` - Derive macros (`#[derive(Actor)]`, `#[messages]`)
- `remote` - Distributed actors via libp2p
- `tracing` - Structured logging
- `metrics` - Prometheus metrics

---

## Commit Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org):
```
feat(actor): add support for remote actors
fix(pubsub): resolve issue with message broadcasting
docs(readme): update installation instructions
```

Add `!` after type for breaking changes: `feat(actor)!: change remote actor messaging system`

---

## Workspace Structure

```
kameo/
├── src/       # Main library
├── macros/    # Procedural macros
├── actors/    # Utility actors
├── examples/  # Usage docs + integration tests
└── benches/   # Benchmarks
```

---

## Verification Before Submitting

1. `cargo fmt --check` (should pass)
2. `cargo clippy` (no warnings)
3. `cargo test` (all tests pass)
4. Public items have documentation
5. `cargo build` (compiles cleanly)

---

## Additional Notes

- Tokio is the default async runtime
- Message passing uses bounded/unbounded channels
- Actors support supervision and fault tolerance
- Remote actors use libp2p for P2P communication
- API docs: https://docs.rs/kameo
- Kameo Book: https://docs.page/tqwewe/kameo
