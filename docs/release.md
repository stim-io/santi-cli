# Release

## Release model

- releases are dispatch-driven beta releases only
- the supported beta version shape is `0.1.0-beta.N`
- the canonical success tag shape is `v0.1.0-beta.N`
- stable tags are rejected
- release artifacts are built from this repo directly
- archives are produced for Linux, macOS, and Windows, with checksums published alongside them
- the release gate rejects skipped tests before `fmt`, `test`, and `clippy`
- the workflow creates the success tag only after the GitHub prerelease is published successfully

## Required release gate

Before publishing, the workflow must pass the same aggregated verification used locally:

```bash
python3 scripts/verify.py
```

## Beta release entrypoint

The canonical beta release script is:

```bash
python3 scripts/release_beta.py preflight --version 0.1.0-beta.N
```

The GitHub workflow `.github/workflows/release-beta.yml` is the protected release entrypoint. It accepts a beta version and ref, verifies the repo state, builds release archives, publishes the GitHub prerelease, and only then creates the matching success tag.

## Artifact direction

Keep the release surface simple:

- compiled CLI binaries
- archive per supported target
- checksums for published artifacts

Do not add installer orchestration or docs-site deployment here unless the repo clearly needs it.
