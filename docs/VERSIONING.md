# Versioning Workflow

This document describes the versioning strategy for EQAPO GUI.

## Semantic Versioning

We use [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes (e.g., incompatible profile format)
- **MINOR**: New features (e.g., new filter types, UI features)
- **PATCH**: Bug fixes and minor improvements

## Version Locations

Version must be updated in **3 files**:

| File | Field |
|------|-------|
| `package.json` | `"version": "x.x.x"` |
| `src-tauri/Cargo.toml` | `version = "x.x.x"` |
| `src-tauri/tauri.conf.json` | `"version": "x.x.x"` |

## Release Workflow

### 1. Prepare Release (on `main`)

```bash
# Update version in all 3 files
# Example: 0.1.0 → 0.2.0

# Commit version bump
git add -A
git commit -m "chore: bump version to 0.2.0"
git push origin main
```

### 2. Create Release

```bash
# Merge main into release branch
git checkout release
git merge main
git push origin release

# This triggers the GitHub Action to build
```

### 3. Tag After Build

```bash
# After build succeeds, tag the release
git tag v0.2.0
git push origin v0.2.0
```

## Pre-release Versions

For alpha/beta releases, use suffixes:
- `0.2.0-alpha.1`
- `0.2.0-beta.1`
- `0.2.0-rc.1` (release candidate)

## Changelog

Maintain `CHANGELOG.md` with:

```markdown
## [0.2.0] - 2026-01-03

### Added
- Feature X

### Fixed
- Bug Y

### Changed
- Improvement Z
```
