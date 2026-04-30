# Release

## Current release posture

- Rust binary release artifacts are deferred.
- The repository still keeps a strict CI shell around the CLI source.
- Reintroduce release packaging only when there is a concrete external distribution need.

## Required guard gate

The required local and CI guard entrypoint is:

```bash
python3 scripts/guard.py
```

## Artifact boundary

Do not add GitHub release archives, checksums, installers, or target-matrix packaging for this Rust binary until distribution is an active requirement.

If that requirement appears, recreate the release path as a focused slice rather than preserving stale packaging scaffolding.
