# Development

## Local checks

Run the aggregated verifier:

```bash
bash scripts/verify.sh
```

Current verification scope:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked -- -D warnings`

## Development rule

When adding behavior, keep the command surface small and land the smallest stable step first.

## Output rule

- human output on stdout by default
- `--json` returns machine-readable results on stdout
- tracing/log output stays on stderr
