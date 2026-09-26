#!/usr/bin/env bash

# Adapted from Rokit (https://github.com/rojo-rbx/rokit), MIT License.
# See LICENSE-rokit.txt in this directory.

set -euo pipefail

CLI_MANIFEST=$(cargo read-manifest --manifest-path Cargo.toml)
CLI_VERSION=$(echo $CLI_MANIFEST | jq -r .version)

echo $CLI_VERSION
