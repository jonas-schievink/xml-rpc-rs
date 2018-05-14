# What to do to publish a new release

1. Ensure all notable changes are in the [changelog](CHANGELOG.md) under "Unreleased".
2. Execute `cargo release -l <level>` to bump version(s), tag and publish everything.
   External subcommand, must be installed with `cargo install cargo-release`.
   
   `<level>` can be one of `major|minor|patch`.
3. Go to GitHub and "add release notes" to the just-pushed tag. Copy them from the changelog.
