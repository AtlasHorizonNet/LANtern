# Changelog

## [0.8.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.7.0...v0.8.0) (2026-07-19)


### Features

* device custom names / rename from the list ([#49](https://github.com/AtlasHorizonNet/LANtern/issues/49)) ([6aee7ce](https://github.com/AtlasHorizonNet/LANtern/commit/6aee7ce6001b51ca09980e24b2513bae4d9756e2))
* Settings About section with version info and source link ([#44](https://github.com/AtlasHorizonNet/LANtern/issues/44)) ([de82c7d](https://github.com/AtlasHorizonNet/LANtern/commit/de82c7d4bbc2e2f8bab7eae1a7ca96b53636cb9c))

## [0.7.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.6.0...v0.7.0) (2026-07-19)


### Features

* split Scan controls from Devices results ([d0f16c5](https://github.com/AtlasHorizonNet/LANtern/commit/d0f16c50476e3bd1773538941ae73a66df7f4773))
* split Scan controls from Devices results ([3689251](https://github.com/AtlasHorizonNet/LANtern/commit/3689251c8f09b4cc764e7e127b848eaf2a30cba5))
* split Scan controls from Devices results ([#41](https://github.com/AtlasHorizonNet/LANtern/issues/41)) ([d0f16c5](https://github.com/AtlasHorizonNet/LANtern/commit/d0f16c50476e3bd1773538941ae73a66df7f4773))
* TCP port scan and Wake-on-LAN ([#43](https://github.com/AtlasHorizonNet/LANtern/issues/43)) ([ca142b6](https://github.com/AtlasHorizonNet/LANtern/commit/ca142b66375276de34fce94e7b36223727d2714b))

## [0.6.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.5.0...v0.6.0) (2026-07-19)


### Features

* DHCP discover tool and fix redacted macOS SSIDs ([a991864](https://github.com/AtlasHorizonNet/LANtern/commit/a991864c5e159c44f7a219a4ae57d003f744a9f5))
* DHCP discover tool and fix redacted macOS SSIDs ([#38](https://github.com/AtlasHorizonNet/LANtern/issues/38)) ([a991864](https://github.com/AtlasHorizonNet/LANtern/commit/a991864c5e159c44f7a219a4ae57d003f744a9f5))
* DHCP discover tool and ignore redacted macOS SSIDs ([8c311c3](https://github.com/AtlasHorizonNet/LANtern/commit/8c311c34c5ae6a4eb14d40a12a7000c836ba3e9b))

## [0.5.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.4.0...v0.5.0) (2026-07-19)


### Features

* network-scoped devices, SQLite history, and network identity ([88a8410](https://github.com/AtlasHorizonNet/LANtern/commit/88a8410b91760c7838b65d6b62ce8a585f009e9a))
* network-scoped devices, SQLite history, and network identity ([#35](https://github.com/AtlasHorizonNet/LANtern/issues/35)) ([88a8410](https://github.com/AtlasHorizonNet/LANtern/commit/88a8410b91760c7838b65d6b62ce8a585f009e9a))
* scope devices by network with SQLite history and identity ([4b6590e](https://github.com/AtlasHorizonNet/LANtern/commit/4b6590e5be4227223ebca7390816514419b51831))


### Bug Fixes

* satisfy clippy needless_return on macOS SSID fallback ([728b13e](https://github.com/AtlasHorizonNet/LANtern/commit/728b13e43f2327590592ec458b652b6504c9de36))

## [0.4.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.3.2...v0.4.0) (2026-07-19)


### Features

* add DNS tools, Settings, and History pages ([4350d05](https://github.com/AtlasHorizonNet/LANtern/commit/4350d058e1431e33a284fc98aa06ce09cbb57fc5))
* add DNS tools, Settings, and History pages ([4ace9f5](https://github.com/AtlasHorizonNet/LANtern/commit/4ace9f54bc1312ee99122786a94f5888c7995255))
* add DNS tools, Settings, and History pages ([#29](https://github.com/AtlasHorizonNet/LANtern/issues/29)) ([4350d05](https://github.com/AtlasHorizonNet/LANtern/commit/4350d058e1431e33a284fc98aa06ce09cbb57fc5))

## [0.3.2](https://github.com/AtlasHorizonNet/LANtern/compare/v0.3.1...v0.3.2) (2026-07-17)


### Bug Fixes

* align device-list scrollbar with the window edge ([1b7a3a1](https://github.com/AtlasHorizonNet/LANtern/commit/1b7a3a13d0bc905e0ab51e7eb922e55f2284db6c)), closes [#22](https://github.com/AtlasHorizonNet/LANtern/issues/22)
* keep interface dropdown text clear of the caret ([95c5bf4](https://github.com/AtlasHorizonNet/LANtern/commit/95c5bf426bdce53570a071e1f7fa8fbb623d1bea)), closes [#21](https://github.com/AtlasHorizonNet/LANtern/issues/21)
* publish a complete latest.json after multi-platform builds ([743a389](https://github.com/AtlasHorizonNet/LANtern/commit/743a3895d436ad2e855e333d5b4ad36b2d28e9dd)), closes [#19](https://github.com/AtlasHorizonNet/LANtern/issues/19)
* resolve open UI, updater, and Windows icon bugs ([b0ec337](https://github.com/AtlasHorizonNet/LANtern/commit/b0ec3372cad1557d7380ff1bd131f9c4219de2b0))
* set NSIS installerIcon to the LANtern .ico ([f8816ce](https://github.com/AtlasHorizonNet/LANtern/commit/f8816ceb6ea8d705834e9cc5c960683563cc072a)), closes [#26](https://github.com/AtlasHorizonNet/LANtern/issues/26)
* use bundled LANtern icon for the Windows taskbar ([9eee67f](https://github.com/AtlasHorizonNet/LANtern/commit/9eee67faa335ff1a3cc56bd06ea470151319b010)), closes [#20](https://github.com/AtlasHorizonNet/LANtern/issues/20)

## [0.3.1](https://github.com/AtlasHorizonNet/LANtern/compare/v0.3.0...v0.3.1) (2026-07-17)


### Features

* update LANtern logo and regenerate app icons ([604c72e](https://github.com/AtlasHorizonNet/LANtern/commit/604c72e8d97b4f0214a613c3b164e80c22c1b3ea))


### Miscellaneous Chores

* prepare release 0.3.1 ([ab6fe24](https://github.com/AtlasHorizonNet/LANtern/commit/ab6fe24e215e0581cbdc09d98341124f06310e57))

## [0.3.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.2.1...v0.3.0) (2026-07-17)


### Features

* ping tool for individual devices ([c4c4467](https://github.com/AtlasHorizonNet/LANtern/commit/c4c4467d2b5477a5d9f6e34851cae4e2932b5417))

## [0.2.1](https://github.com/AtlasHorizonNet/LANtern/compare/v0.2.0...v0.2.1) (2026-07-17)


### Bug Fixes

* sync updater pubkey in tauri.conf.json after key rotation ([9c9d61f](https://github.com/AtlasHorizonNet/LANtern/commit/9c9d61ffeb66af4501230823f12f0e8efd6f77bf))

## [0.2.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.1.0...v0.2.0) (2026-07-17)


### Features

* allow choosing which network interface/IP to scan ([fd22159](https://github.com/AtlasHorizonNet/LANtern/commit/fd22159fcf722ce355159aaaf2da9002285b27ad))
* in-app self-update from GitHub releases + selectable scan interface ([4ba7d56](https://github.com/AtlasHorizonNet/LANtern/commit/4ba7d568fb70cb16087922be62c58e9b9f8ec76a))
* self-update from GitHub releases ([707aec2](https://github.com/AtlasHorizonNet/LANtern/commit/707aec22cf54385c96ddbc25cc02026a5f723337))

## [0.1.0](https://github.com/AtlasHorizonNet/LANtern/compare/v0.1.0...v0.1.0) (2026-07-17)


### Miscellaneous Chores

* bootstrap release 0.1.0 ([d609422](https://github.com/AtlasHorizonNet/LANtern/commit/d609422d798cab3974d50252dcdcf760d35a734d))
