# Cloak v0.4.0 Handoff

This document records what shipped in `v0.4.0`, the current verification bar, and the remaining release operations that are intentionally outside this PR-ready branch.

## Summary

`v0.4.0` closes the six follow-up items deferred after `v0.3.0`:

- versioned wire format detection and rejection of unknown versions
- optional rayon-backed parallel embedding
- detailed capacity reporting
- GIF-specific steganalysis
- property-based test coverage
- cargo-fuzz harnesses

The workspace is versioned at `0.4.0` across `cloak-core`, `cloak-cli`, and `cloak-tui`. Only `cloak-core` is publishable; the CLI and TUI remain workspace binaries with `publish = false`.

## Shipped Changes

### 1. Versioned Wire Format

- Added `WireVersion`, `detect_version()`, and `CloakError::UnsupportedVersion`.
- Kept the existing `v1` encrypted payload layout unchanged; the new dispatch layer makes future versions explicit and safely rejectable.

### 2. Parallel Embedding

- Added `EmbedOptions.parallel`, CLI `--parallel`, and `embed_lsb_parallel()`.
- Parallel mode is an internal optimization. It preserves the encrypted payload and wire format, and falls back for small images.

### 3. Detailed Capacity Reporting

- Added `CapacityReport` with nested `CapacityBreakdown`.
- CLI `capacity` output now shows total pixel bits, header overhead, block alignment effects, crypto overhead, and final usable bytes.

### 4. GIF-Specific Steganalysis

- Added palette anomaly detection, EzStego detection, gifshuffle detection, and palette chi-square analysis via `analysis::analyze_gif()`.
- CLI `analyze` prints the GIF-specific results when the input is a GIF.

### 5. Test and Fuzz Hardening

- Added five proptest suites covering roundtrip, analysis invariants, capacity, crypto, and format detection.
- Added six cargo-fuzz targets covering extract, embed, analyze, decrypt, format detection, and wire format parsing.

## Repo State

- `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, and `SECURITY.md` now describe the `0.4.x` line and current workflow.
- `docs/superpowers/specs/2026-03-16-v0.4.0-roadmap-design.md` is included as the design-history record for this release.
- The CI and PR gate is `make check`, which runs:

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

- `cargo bench --package cloak-core` remains the performance check when embedding code changes.
- Nightly fuzz runs are available for hardening but are not part of the default merge gate.

## Remaining Release Operations

- Open the `v0.4.0` PR using the existing pull request template and the `make check` results.
- If a public release is desired after merge, publish `cloak-core`, tag the repo, and draft GitHub release notes.

Those operations are intentionally outside this branch. This branch is PR-ready release prep, not the public publish step.
