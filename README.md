<div align="center" width="800">
    <img alt="Wally Logo" src="wally-logo.svg" />
    <h1>Wally, a package manager for Roblox</h1>
</div>

* [Installation](#installation)
* [Commands](#commands)
* [Prior Art](#prior-art)
* [Manifest Format](#manifest-format)
* [Lockfile Format](#lockfile-format)
* [Registries](#registries)
* [License](#license)

## About
Wally is a package manager for Roblox inspired by Cargo (Rust) and npm (JavaScript). It brings the familiar, community-oriented world of sharing code from other communities into the Roblox ecosystem.

Wally has two pieces that work together: a command line tool named `wally` and a registry server that hosts packages. Most users will only interact with the command line tool, but both are available in this repository.

## Installation

### From GitHub
Pre-built binaries are available for Windows, macOS, and Linux from the [GitHub Releases Page for Wally][releases].

[releases]: https://github.com/UpliftGames/wally/releases

### With Foreman
[Foreman][foreman] is a toolchain manager developed for the Roblox community. You can use it to install Wally:

``` toml
wally = { source = "UpliftGames/wally", version = "0.2.1" }
```

[foreman]: https://github.com/Roblox/foreman

### From Source
It's straightforward to compile Wally from source. Wally requires Rust 1.51.0 or newer.

Clone the repository and use:

```bash
cargo install --locked --path .
```

## Commands

### `wally init`
Create a new, empty package.

Parity with:
* `cargo init`
* `npm init`

### `wally install [--locked]`
Installs all packages.

`--locked` matches `cargo XXX --locked`, which will error if there is not an up-to-date lockfile. Intended for use on CI machines.

Parity with:
* `npm install` with no arguments

### `wally update [package-names]` (unimplemented)
Update packages recursively. By default, will update all packages. If any package names are given (in the form `scope/name` or `scope/name@version-req`), just those packages will be updated instead.

Parity with:
* `cargo publish`
* `npm update` (npm 7+, equivalent to `--depth 9999` in npm 6.x and older)

### `wally publish`
Publish the current package.

Parity with:
* `cargo publish`
* `npm publish`

### `wally login`
Log into an account to publish packages to a registry.

Partiy with:
* `cargo login`
* `npm login`

### `wally logout`
Log out of a registry account.

Parity with:
* `cargo logout`
* `npm logout`

### `wally package --output <path>`
Package the current project as a zip file suitable for uploading to the package registry. Useful for adding entries to the registry and debugging what ends up in the blob that will be uploaded.

Parity with:
* `cargo package`

### `wally manifest-to-json`
Prints the current project's manifest as a line of JSON. Used for adding entries to the package index.

Parity with:
* `cargo read-manifest`

## Prior Art
Wally aims to stand on the shoulders of giants. Decisions we make are in part backed up by looking at other package managers and other public documentation:

* [*So you want to write a package manager*](https://medium.com/@sdboyer/so-you-want-to-write-a-package-manager-4ae9c17d9527)
* [crates.io](https://crates.io/) and [Cargo](https://github.com/rust-lang/cargo) from the Rust ecosystem
* [npm](https://npmjs.org/) from the JavaScript ecosystem
* [PyPI](https://pypi.org/), [pip](https://pip.pypa.io/en/stable/), [pipenv](https://pypi.org/project/pipenv/), and [Poetry](https://python-poetry.org/) from Python

## Manifest Format
The package manifest file describes a package and all of the packages it depends on. Package manifests are written in [TOML][toml] and stored in a file named `wally.toml`.

Manifest files are written by humans. They can contain comments and formatting decisions that are tough to preserve with automatic editing tools. This should be okay -- editing a package manifest should be easy.

Manifest files define all necessary information about a package.

Here is an example package manifest, annotated with comments:

```toml
[package]
# Package names are always "SCOPE/NAME"
# They can inclde lowercase letters, numbers, and dashes.
name = "lpghatguy/asink"

# Descriptions are free-form. These will be used as part of package listings
# and search results.
description = "Asynchronous programming primitives"

# Versions follow Semantic Versioning.
# https://semver.org/
version = "2.0.7"

# Contains an SPDX License Expression.
# Licenses are required for publishing code to public registries.
license = "MIT OR Apache-2.0"

# The author list is a free-form list, but conventionally contains names and
# email addresses.
authors = ["Lucien Greathouse <lucien@uplift.games>"]

# Packages belong to a "realm", which helps prevent using code in the wrong
# context. For now, we have "server" and "shared" realms.
realm = "shared"

# Wally supports multiple registries.
# This feature can be used to have split public/private registries to
# keep internal code private and isolated.
registry = "https://github.com/upliftgames/wally-index"

[dependencies]
# Most dependencies will look like this.
#
# The name on the left is an alias. It defines what name we would like to
# use to refer to this package.
#
# The value on the right will usually be a string of the form
#   "SCOPE/NAME@VERSION_REQ"
# Versions are SemVer version requirements. The default behavior matches
# Cargo, or npm with the `^` version specifier.
Roact = "roblox/roact@1.2.0"
Promise = "evaera/promise@2.0.1"

# In the future, it'll be possible to pull dependencies from Git.
#
# This shouldn't be used for most dependencies. It's intended to be used for
# testing changes from PRs or other experimental branches.
CoolThing = { git = "https://github.com/Roblox/cool-thing.git", branch = "pr-1231" }

# Dependencies will optionally also contain a file path. This feature is
# based on a similar feature in Cargo that is useful for having multiple
# packages in the same repository, like the client and server halves of the
# same codebase.
MonoThing = { registry = "roblox/mono-thing@1.3.2", path = "../MonoThing" }

# Any dependency that we use only for testing goes in a special section.
#
# These dependencies will not be shipped with the production version of a
# project.
[dev-dependencies]
TestEZ = "roblox/testez@1.6.3"
```

## Lockfile Format
The lockfile contains the exact versions of each dependency that a project depends on. They're a critical feature that ensures that everyone who works on a game is getting the exact same version of every package.

Lockfiles are written in [TOML][toml] and stored in a file named `wally.lock`. They're are human readable, but are only written by tools. We've optimized the lockfile format for reading as well as diffing so that they're as easy to review as possible.

```toml
[[package]]
name = "registry:lpghatguy/asink"
version = "2.0.7"
dependencies = [
 "registry:roblox/roact",
 "registry:evaera/roblox-lua-promise",
 "registry:roblox/mono-thing",
 "git:https://github.com/Roblox/cool-thing.git",
]

[[package]]
name = "registry:evaera/roblox-lua-promise"
version = "2.1.0"
checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[[package]]
name = "registry:roblox/mono-thing"
version = "1.3.2"
checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[[package]]
name = "git:https://github.com/Roblox/cool-thing.git"
rev = "foo"
commit = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
```

## Registries
Like many programming language package managers, Wally packages are published to a registry.

A Wally registry consists of two pieces, inspired by Cargo and crates.io:
* A Git repository containing a package index
* A registry API that handles downloading and publishing package contents

We do not currently host an official Wally registry. This will be coming eventually!

### Registry API
Authentication is currently done with simple tokens. This isn't feasible for hosting a publicly-writable registry.

* GET `/v1/package-contents/<scope>/<name>/<version>`
	* Returns the contents of a package for installation
	* Package contents are ZIP files
* POST `/api/v1/publish`
	* Client will post a package tarball that is extracted and published from the server.

[toml]: https://toml.io/

## License
Wally is available under the terms of the Mozilla Public License Version 2.0. Terms and conditions are available in [LICENSE.txt](LICENSE.txt) or at <https://www.mozilla.org/en-US/MPL/2.0/>.
