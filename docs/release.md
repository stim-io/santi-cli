# Release

## Release model

- releases are tag-driven beta releases only
- the supported tag shape is `v0.1.0-beta.N`
- stable tags are rejected
- release artifacts are built from this repo directly
- archives are produced for Linux, macOS, and Windows, with checksums published alongside them
- the release gate rejects skipped tests before `fmt`, `test`, and `clippy`

## Required release gate

Before publishing, the workflow must pass the same aggregated verification used locally:

```bash
bash scripts/verify.sh
```

## Artifact direction

Keep the release surface simple:

- compiled CLI binaries
- archive per supported target
- checksums for published artifacts

Do not add installer orchestration or docs-site deployment here unless the repo clearly needs it.
