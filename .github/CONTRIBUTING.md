# Contributing to Kruk Screenshots

Contributions, bug reports, and suggestions are welcome!

## Development

Kruk Screenshots is a cross-platform screenshot utility written in Rust.

You'll need a recent stable Rust toolchain and the native libraries required by your operating system.

Before submitting a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

The **Build check** GitHub Actions workflow can also be triggered manually to verify compilation across supported architectures.

## Creating a release

Releases are automated through GitHub Actions.

1. Update the version in `Cargo.toml`.
2. Update `Cargo.lock` by running `cargo check` and commit both files.
3. Make sure the changes are merged into the main branch and the build checks pass.
4. Create and push a version tag:

   ```bash
   git tag v0.8.1
   git push origin v0.8.1
   ```

5. The **Release** workflow automatically:
   - Creates a draft GitHub Release.
   - Builds binaries for Linux, macOS, and Windows across supported architectures.
   - Uploads the release archives.
   - Verifies that all expected assets are present.
   - Publishes the release only after all builds succeed.

### Important notes

- Do not create or publish the GitHub Release manually before pushing the tag.
- If a build fails, the release remains a draft. Investigate the failure before retrying.
- Published releases are immutable: their assets and associated tags cannot be replaced. Fixes require a new version and tag.
- Never reuse an existing published release tag.

## License

By contributing, you agree that your contributions will be distributed under the project's license.