# AGENTS

## Purpose

This repository owns the standalone `santi-cli` public release unit.

- It is public so releases can be distributed cleanly.
- It is not yet a user-facing product-docs repository.
- `AGENTS.md` and `docs/` are the documentation truth source.

## Repository Boundary

- Keep this repo focused on the CLI binary, release packaging, and its immediate docs.
- Do not add a docs site, README-first product surface, or community-facing content workflow.
- Do not pull broader `santi` runtime ownership into this repo.
- Prefer one small Rust workspace with one primary `app/` crate until real growth forces a split.
- Keep the public command surface centered on core runtime use: `health`, `chat`, `soul`, and `session`.
- Do not add service-admin commands unless this repo explicitly broadens into an operator tool.

## Stable CLI Direction

- HTTP-only client with `--base-url` overridable by CLI flag, environment, then config.
- Current migration slice also includes human-default rendering for the core `soul` and `session` commands over HTTP.
- Output is human-friendly by default.
- `--json` is the machine-readable output path.
- `--log-level` configures CLI-side `tracing`.
- CLI logs go to stderr; command results go to stdout.
- Runtime hook replacement remains an admin/API operation, not part of the standalone CLI primary surface.

## Quality Bar

- Keep CI minimal but strict: format, test, clippy.
- Keep release flow tag-driven and artifact-oriented.
- Prefer the smallest working scaffold over speculative architecture.

## Git Strategy

- Default PR integration strategy is **squash + delete branch**.
- Treat merged feature branches as disposable after squash merge.

## Key File Index

- `AGENTS.md`: stable repository boundary and file index
- `docs/core.md`: top-level repository model and command-surface direction
- `docs/dev.md`: local development and verification rules
- `docs/release.md`: release and packaging expectations
- `app/src/main.rs`: current CLI scaffold entrypoint
- `scripts/verify.sh`: aggregated local/CI verification entrypoint
- `.github/workflows/ci.yml`: required continuous integration checks
- `.github/workflows/release.yml`: tag-driven release workflow

## Update Rules

- Keep `AGENTS.md` short and durable.
- Put design reasoning and evolving decisions in `docs/`.
- Only add new top-level files when they materially improve release, verification, or boundary clarity.
