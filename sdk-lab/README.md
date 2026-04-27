# Huefy Rust SDK Lab

A verification runner for the Huefy Rust SDK implemented as a Cargo example.

## Scenarios

1. **Initialization** — create client with a dummy key, verify no error
2. **Config validation** — empty API key returns an error
3. **HMAC signing** — sign payload with HMAC-SHA256, verify 64-char hex result
4. **Error sanitization** — email redacted from error messages
5. **PII detection** — email and SSN fields detected in field list
6. **Circuit breaker state** — new circuit breaker starts in Closed state
7. **Health check** — invoke `GET /health` against the configured base URL
8. **Cleanup** — close client gracefully

## Run

From `sdks/rust/`:

```bash
cargo run --example sdk-lab
```
