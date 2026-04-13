# Core

## Role

`santi-cli` is the standalone CLI release unit for the `santi` runtime.

This repo is public for source visibility and release distribution, not because it already needs a full external-docs surface.

## Current Shape

- single primary crate in `app/`
- current migration slice: config resolution, HTTP client plumbing, `health`, `chat`, `soul get`, `soul memory set`, and session `create|get|fork|send|watch|messages|effects|compact|compacts|memory get|memory set` over HTTP, with human-friendly default output for the core result-bearing commands

## Command Surface Direction

- config precedence: CLI > env > config file > defaults
- default target URL: `http://127.0.0.1:18081`
- output default: human-friendly
- machine-readable mode: `--json`
- streamed event mode: `--raw` on chat/send, mutually exclusive with `--json`
- long-running human observation: `session watch <id>` follows the runtime-owned watch snapshot + SSE path, emits single-line `::` metadata records, and wraps message bodies in explicit `:: content_begin` / `:: content_end` boundaries instead of dumping raw protocol events
- CLI diagnostics: `--log-level` backed by `tracing`
- the public command surface is intentionally centered on core runtime use, not service administration

## Non-Goals Right Now

- no attempt to move the whole `santi` runtime here
- no public tutorial/documentation site
- no expanded multi-crate layering until the CLI actually needs it
- no admin hooks reload in the standalone CLI primary surface

## Explicit Boundary Choice

- `santi-cli` intentionally does not expose service-admin hook management in its primary public command surface.
- Runtime hook replacement remains an admin/API operation (`PUT /api/v1/admin/hooks`), not a standard standalone CLI workflow.
- This is a deliberate boundary choice: the standalone CLI focuses on core runtime usage and avoids backend-specific operator commands.
