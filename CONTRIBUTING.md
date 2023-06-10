# Contribution guide for wally

More info will be added here in the future. For now, if you want changes then please add an issue or create a PR! All issues, PRs, and comments on PRs are incredibly helpful. It may take us a while to get to your PR but if you feel it is important then please head to the #wally channel in the Roblox OSS discord server (find this in the top right of [wally.run](https://wally.run/)) and give us a ping!

## Creating a new wally release

1. Change versions in 
    - `Cargo.toml`
    - `README.md`
    - `wally-registry-backend/Cargo.toml`
    - `wally-registry-frontend/package.json`
    - `wally-registry-frontend/src/pages/Install.mdx`
2. Run these commands to ensure the lockfiles are updates and tests pass
    - `cargo test`
    - `npm --prefix wally-registry-frontend i --package-lock-only`
3. Update `CHANGELOG.md` to reflect what has been added to the new release
4. Commit `git add . && git commit -m "Release vX.Y.Z"`
5. Tag `git tag vX.Y.Z`
6. Publish to crates.io `cargo publish`
7. Push `git push && git push --tags`
8. Update release information

(Thank you to lpg / rojo for inspiration for this release checklist)