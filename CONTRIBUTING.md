# Contribution guide for wally

Please respect that wally is still in early stages of development. Things are messy. Time is precious. We may be slow.

- **Only start making a PR if you are confident with Rust or TS/React** (issues and comments are always appreciated though)
- **Documentation for developing exists but may be outdated**
  - **Be prepared to figure out how to get things going**
  - **Fixing up any issues you do have getting going is a fantastic way to start helping!**
- **Polish and refine PRs as much as possible and ensure they pass CI checks before requesting a review**
- **If you want advise on a draft then discuss it in [#wally] first.** 
  
Beyond that we are pretty chill, I promise! If you make good changes I will do my best to help you get them to the finish line.

More info will be added here in the future. For now, if you want changes then please add an issue or create a PR! All issues, PRs, and comments on PRs are incredibly helpful. It may take us a while to get to your PR but if you feel it is important then please head to the [#wally] channel in the Roblox OSS discord server (find this in the top right of [wally.run](https://wally.run/)) and let us know!

The current lead maintainer for wally is @magnalite, that's me! If you want to work on a complex change or feel like your pr/issue has gone unnoticed for too long then give me a ping in [#wally]!

Finally, as you may have guessed by now... If in doubt head to [#wally] and ask. Anyone is welcome to come in and ask anything about wally.

[#wally]: https://discord.com/channels/385151591524597761/872225914149302333

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
