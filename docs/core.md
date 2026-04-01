# Core

## Role

`santi-cli` is the standalone CLI release unit for the `santi` runtime.

This repo is public for source visibility and release distribution, not because it already needs a full external-docs surface.

## Current Shape

- root Rust workspace
- single primary crate in `app/`
- no docs site or node-based docs toolchain
- no broad runtime migration yet

## Command Surface Direction

- backend selector vocabulary: `http|local`
- default backend: `http`
- output default: human-friendly
- machine-readable mode: `--json`
- CLI diagnostics: `--log-level` backed by `tracing`

## Non-Goals Right Now

- no attempt to move the whole `santi` runtime here
- no public tutorial/documentation site
- no expanded multi-crate layering until the CLI actually needs it
