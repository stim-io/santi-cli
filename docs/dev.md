# Development

## Local checks

Run the aggregated verifier:

```bash
bash scripts/verify.sh
```

Current verification scope:

- skipped tests are rejected before the Rust checks run
- `cargo fmt --check`
- `cargo test --locked`

Current CLI slice:

- `santi-cli health`
- `santi-cli chat <message>` or stdin message input
- `santi-cli soul get`
- `santi-cli soul memory set` from stdin
- `santi-cli session create|get|fork|send|messages|effects|compact|compacts|memory get|memory set`
- `--base-url`, `--json`, and `--log-level`
- `chat --raw` and `session send --raw` stream event-shaped output; `--json` returns a summarized machine-readable result
- `--raw` and `--json` are mutually exclusive on `chat` and `session send`
- `soul get`, `soul memory set`, `session get`, `session fork`, `session compact`, `session compacts`, `session memory get|set`, and `session messages` default to human-readable output; `--json` preserves structured output
- `local` currently errors as not implemented
- admin hook reload is intentionally excluded from this standalone CLI surface

## Development rule

When adding behavior, keep the command surface small and land the smallest stable step first.

## Output rule

- human output on stdout by default
- `--json` returns machine-readable results on stdout
- tracing/log output stays on stderr

## Release packaging

Release archives are beta-only (`v0.1.0-beta.N`) and expected for Linux, macOS, and Windows targets, with checksums published for each packaged artifact.
