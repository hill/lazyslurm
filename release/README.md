# Release & Distribution

This directory contains files related to distributing LazySlurm.

## Directory Structure

- `homebrew/` - Homebrew formula for macOS/Linux installation
- `packaging/` - Future: RPM/DEB package specs
- `scripts/` - Future: Installation scripts

## Release Process

### 1. Create a Release

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with changes
3. Create and push a git tag:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
4. GitHub Actions will automatically:
   - Build cross-platform binaries
   - Create GitHub release
   - Upload binary assets
   - Publish to crates.io

### 2. Update Homebrew Formula

After a successful release:

1. Update URLs and SHA256 hashes in `homebrew/lazyslurm.rb`
2. Test the formula locally:
   ```bash
   brew install --build-from-source ./release/homebrew/lazyslurm.rb
   ```
3. Submit to homebrew-core or create a tap

### 3. Distribution Checklist

- [ ] GitHub release with binaries
- [ ] Published to crates.io
- [ ] Homebrew formula updated
- [ ] README installation instructions updated
- [ ] Test installations on different platforms

## Homebrew Tap (Future)

To create your own tap:

```bash
# Create tap repository
gh repo create homebrew-tap --public

# Add formula
cp release/homebrew/lazyslurm.rb /path/to/homebrew-tap/Formula/

# Users install with:
brew tap hill/tap
brew install lazyslurm
```