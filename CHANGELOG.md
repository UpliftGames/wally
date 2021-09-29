# Wally Changelog

## Unreleased Changes

## 0.2.0 (2021-09-29)
* Fixed package links between sibling packages. ([#17][#17])
* Fixed invalid package names being generated from `wally init`. ([#19][#19])
* Added public authentication strategy via GitHub. ([#20][#20])
* Improved readability of package name errors. ([#23][#23])
* Renames rojo projects when the name given in `default.project.json` doesn't match the name given in `wally.toml`. ([#25][#25])
* More efficient use of reqwest. ([#27][#27])
* Enforce a max length on package names. ([#28][#28])

[#17]: https://github.com/UpliftGames/wally/pull/17
[#19]: https://github.com/UpliftGames/wally/pull/19
[#20]: https://github.com/UpliftGames/wally/pull/20
[#23]: https://github.com/UpliftGames/wally/pull/23
[#25]: https://github.com/UpliftGames/wally/pull/25
[#27]: https://github.com/UpliftGames/wally/pull/27
[#28]: https://github.com/UpliftGames/wally/pull/28

## 0.1.1 (2021-08-03)
* Fixed `wally install` failing due to a "file not found" error. ([#11][#11])

[#11]: https://github.com/UpliftGames/wally/pull/11

## 0.1.0 (2021-08-03)
* Initial release
