<div align="center" width="800">
    <img alt="Wally Logo" src="wally-logo.svg" />
    <h1>Wally, a package manager for Roblox
    	<br><a href="https://upliftgames-wally.statuspage.io/"><img alt="Dynamic Status Badge" src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fvhs2807lh6ws.statuspage.io%2Fapi%2Fv2%2Fsummary.json&query=%24.status.description&label=Status%20"></a>
	</h1>
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

### With Aftman (preferred)
[Aftman][aftman] is the toolchain manager we recommend. You can use it to install Wally:

In your project (preferred)
```bash
aftman init
aftman add UpliftGames/wally
aftman install
```

Or install wally globally
```bash
aftman add --global UpliftGames/wally
aftman install
```

[aftman]: https://github.com/LPGhatguy/aftman

### Homebrew
[Homebrew][homebrew] is also a convenient way to install wally! (only for MacOS/Linux)
```bash
brew install wally
```

[homebrew]: https://brew.sh/

### From GitHub
Pre-built binaries are available for Windows, macOS, and Linux from the [GitHub Releases Page for Wally][releases].

[releases]: https://github.com/UpliftGames/wally/releases

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
(locked is a planned feature and not yet implemented)

Parity with:
* `npm install` with no arguments

### `wally update [package-names]`
Update packages recursively. By default, will update all packages. If any package names are given (in the form `scope/name` or `scope/name@version-req`), just those packages will be updated instead.

Parity with:
* `cargo update`
* `npm update` (npm 7+, equivalent to `--depth 9999` in npm 6.x and older)

### `wally publish [--token <token>]`
Publish the current package.

Parity with:
* `cargo publish`
* `npm publish`

### `wally login [--token <token>]`
Log into an account to publish packages to a registry.

You can also directly provide a token via `wally login --token "$WALLY_AUTH_TOKEN"`.

Parity with:
* `cargo login`
* `npm login`

### `wally logout`
Log out of a registry account.

Parity with:
* `cargo logout`
* `npm logout`

### `wally package [--list] --output <path>`
Package the current project as a zip file suitable for uploading to the package registry. Useful for adding entries to the registry and debugging what ends up in the blob that will be uploaded. `--list` will output which files will be included instead of creating a zip file.

Parity with:
* `cargo package`

### `wally manifest-to-json`
Prints the current project's manifest as a line of JSON. Used for adding entries to the package index.

Parity with:
* `cargo read-manifest`

### `wally search <query>`
Search the registry to see what packages are available.

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
# They can include lowercase letters, numbers, and dashes.
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
# The server realm should only be used for packages which shouldn't be replicated.
realm = "shared"

# Wally supports multiple registries.
# This feature can be used to have split public/private registries to
# keep internal code private and isolated.
registry = "https://github.com/upliftgames/wally-index"

# You can also specify files to include or exclude from the package
# By default gitignore files are respected and Wally won't include hidden
# files/directories or packages downloaded by Wally.
# include = []
exclude = ["node_modules"]

# Packages can be marked as private to prevent them from being published.
private = true

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

[server-dependencies]
# Dependencies in the server realm can be required here as shown above.
# These are dependencies which should only ever exist on the server.

[dev-dependencies]
# Dev dependencies can be server or shared but are only needed during development.
TestEZ = "roblox/testez@0.4.1"
```

## Lockfile Format
The lockfile contains the exact versions of each dependency that a project depends on. They're a critical feature that ensures that everyone who works on a game is getting the exact same version of every package.

Lockfiles are written in [TOML][toml] and stored in a file named `wally.lock`. They're human-readable, but are only written by tools. We've optimized the lockfile format for reading as well as diffing so that they're as easy to review as possible.

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

The official Wally registry is available at https://github.com/upliftgames/wally-index.

### Registry API

* GET `/v1/package-contents/<scope>/<name>/<version>`
	* Returns the contents of a package for installation
	* Package contents are ZIP files
* GET `/v1/package-metadata/<scope>/<name>`
	* Returns metadata for a package
* GET `/v1/package-search?query=phrase`
	* Query what packages are available on this registry
* POST `/api/v1/publish`
	* Client will post a package tarball that is extracted and published from the server.

[toml]: https://toml.io/

## License
Wally is available under the terms of the Mozilla Public License Version 2.0. Terms and conditions are available in [LICENSE.txt](LICENSE.txt) or at <https://www.mozilla.org/en-US/MPL/2.0/>.
