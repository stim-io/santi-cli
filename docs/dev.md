# Development

## Local checks

Run the aggregated verifier:

```bash
python3 scripts/verify.py
```

Current verification scope:

- skipped tests are rejected before the Rust checks run
- `cargo fmt --check`
- `cargo clippy --locked --all-targets`
- `cargo test --locked`

Current CLI slice:

- `santi-cli health`
- `santi-cli chat <message>` or stdin message input
- `santi-cli soul get`
- `santi-cli soul memory set` from stdin
- `santi-cli session create|get|fork|send|watch|messages|effects|compact|compacts|memory get|memory set`
- `--base-url`, `--json`, and `--log-level`
- `chat --raw` and `session send --raw` stream event-shaped output; `--json` returns a summarized machine-readable result
- `--raw` and `--json` are mutually exclusive on `chat` and `session send`
- `session watch <id>` is the human-oriented long-running observation path; it loads a watch snapshot, then follows runtime-owned watch SSE events, emits `::` metadata lines, and places message bodies inside explicit content boundaries so CLI metadata and `santi` content stay visually separate
- `soul get`, `soul memory set`, `session get`, `session fork`, `session compact`, `session compacts`, `session memory get|set`, and `session messages` default to human-readable output; `--json` preserves structured output
- `local` currently errors as not implemented
- admin hook reload is intentionally excluded from this standalone CLI surface

## Development rule

When adding behavior, keep the command surface small and land the smallest stable step first.

## Output rule

- human output on stdout by default
- `--json` returns machine-readable results on stdout
- tracing/log output stays on stderr

## Release posture

Rust binary release artifacts are not published at the current stage. Keep local and CI validation centered on `python3 scripts/verify.py` until an external distribution need justifies rebuilding a release path.
