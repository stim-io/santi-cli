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

## Stable CLI Direction

- Backend vocabulary is `http|local`.
- Default backend is `http`.
- Backend configuration should remain overridable by CLI flag, environment, then config.
- Output is human-friendly by default.
- `--json` is the machine-readable output path.
- `--log-level` configures CLI-side `tracing`.
- CLI logs go to stderr; command results go to stdout.

## Quality Bar

- Keep CI minimal but strict: format, test, clippy.
- Keep release flow tag-driven and artifact-oriented.
- Prefer the smallest working scaffold over speculative architecture.

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
