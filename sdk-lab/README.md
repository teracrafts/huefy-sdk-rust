# Huefy Rust SDK Lab

Verifies the core email contract through the real Rust email client against a local stub server.

## Run

```bash
cargo run --example sdk-lab
```

from `sdks/rust/`.

## Scenarios

1. Initialization
2. Single-send contract shaping
3. Bulk-send contract shaping
4. Validation rejection for invalid single input
5. Validation rejection for invalid bulk input
6. SDK health path behavior
7. Cleanup

## Notes

- The lab uses a loopback stub server rather than the live API.
- It verifies request normalization, parsed responses, and validation boundaries.
