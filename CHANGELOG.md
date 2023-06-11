# Wally Changelog

## Unreleased Changes

## 0.3.2 (2023-06-05)
* Added private field to package manifest ([#9])([#76])
* Improved lockfile generation **lockfiles will change** ([#130])
* Improved package installation to download packages in parallel ([#133])
* Support for directly providing token to `wally login` ([#105][#105])
* Fixed dev dependency resolution ([#75])
* Fixed multi-realm dependency resolution ([#80])
* Fixed dev dependency link generation ([#131])
* Reclaiming of scopes is now disallowed ([#102])
* Improved manifest generation on `wally init` ([#84])
* Added S3 storage backend option ([#90])
* Frontend improvements ([#109])([#110])([#112])
* Publishing packages >2MB now bails at the cli ([#121])
* Other small misc changes related to repo upkeep

[#9]: https://github.com/UpliftGames/wally/issues/9
[#105]: https://github.com/UpliftGames/wally/issues/105
[#75]: https://github.com/UpliftGames/wally/pull/75
[#80]: https://github.com/UpliftGames/wally/pull/80
[#131]: https://github.com/UpliftGames/wally/pull/131
[#102]: https://github.com/UpliftGames/wally/pull/102
[#76]: https://github.com/UpliftGames/wally/pull/76
[#84]: https://github.com/UpliftGames/wally/pull/84
[#90]: https://github.com/UpliftGames/wally/pull/90
[#109]: https://github.com/UpliftGames/wally/pull/109
[#110]: https://github.com/UpliftGames/wally/pull/110
[#112]: https://github.com/UpliftGames/wally/pull/112
[#130]: https://github.com/UpliftGames/wally/pull/130
[#133]: https://github.com/UpliftGames/wally/pull/133
[#121]: https://github.com/UpliftGames/wally/pull/121

## 0.3.1 (2021-11-12)
* Support for dev dependencies ([#63][63])
* Clarified upgrade response for outdated cli clients
* Clarified feedback on failure to resolve package

[#63]: https://github.com/UpliftGames/wally/pull/63

## 0.3.0 (2021-11-11)
* Fixed Windows generating invalid zip archives. ([#15][#15])
* Support for registry fallback chains ([#35][#35])
* Generate cross realm package links ([#38][#38])
* Support include/exclude lists in manifest ([#44][#44])
* Backend API for package metadata and search ([#46][#46])
* Arch users can now use vendored-libgit2 btw ([#52][#52])
* Frontend can search packages and display package info ([#55][#55])
* Minimum Wally version enforcement by registry ([#57][#57])

[#15]: https://github.com/UpliftGames/wally/issues/15
[#35]: https://github.com/UpliftGames/wally/pull/35
[#38]: https://github.com/UpliftGames/wally/pull/38
[#44]: https://github.com/UpliftGames/wally/pull/44
[#46]: https://github.com/UpliftGames/wally/pull/46
[#52]: https://github.com/UpliftGames/wally/pull/52
[#55]: https://github.com/UpliftGames/wally/pull/55
[#57]: https://github.com/UpliftGames/wally/pull/57

## 0.2.1 (2021-10-01)
* First iteration of wally frontend. ([#32][#32])

[#32]: https://github.com/UpliftGames/wally/pull/32

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
