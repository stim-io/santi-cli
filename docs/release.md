# Release

## Release model

- releases are tag-driven
- the initial trigger shape is `v*`
- release artifacts are built from this repo directly

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
