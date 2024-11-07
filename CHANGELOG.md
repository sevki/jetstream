# Changelog

## [5.4.2](https://github.com/sevki/jetstream/compare/v5.4.1...v5.4.2) (2024-11-07)


### Bug Fixes

* auto-release ([ffb2ffc](https://github.com/sevki/jetstream/commit/ffb2ffcd7710f56d3c20a93659fa8c9f6c70417b))

## [5.4.1](https://github.com/sevki/jetstream/compare/v5.4.0...v5.4.1) (2024-11-07)


### Bug Fixes

* readme ([dc77722](https://github.com/sevki/jetstream/commit/dc777223fe95ea79866320b7831e4e3c89631820))

## [5.4.0](https://github.com/sevki/jetstream/compare/v5.3.0...v5.4.0) (2024-11-07)


### Features

* revamp service ([#147](https://github.com/sevki/jetstream/issues/147)) ([6d96be8](https://github.com/sevki/jetstream/commit/6d96be89affc75b9e534122ac9305257861be706))

## [5.3.0](https://github.com/sevki/jetstream/compare/v5.2.3...v5.3.0) (2024-10-23)


### Features

* use sccache ([#142](https://github.com/sevki/jetstream/issues/142)) ([89f96ab](https://github.com/sevki/jetstream/commit/89f96abf5b0527d45bf75e00fafd4cd197fff1d6))


### Bug Fixes

* update release flow ([#144](https://github.com/sevki/jetstream/issues/144)) ([36dd4af](https://github.com/sevki/jetstream/commit/36dd4af7686d405eeaa42d53e7eaac934d901489))

## [5.2.3](https://github.com/sevki/jetstream/compare/v5.2.2...v5.2.3) (2024-10-10)


### Bug Fixes

* bump zero copy ([#140](https://github.com/sevki/jetstream/issues/140)) ([4bb933f](https://github.com/sevki/jetstream/commit/4bb933f8ab6399e44e9a9ba4dc9ff92b43d69454))
* unused git ([3fe908e](https://github.com/sevki/jetstream/commit/3fe908e405bc870a8089b18e88dae245f41eeb68))

## [5.2.2](https://github.com/sevki/jetstream/compare/v5.2.1...v5.2.2) (2024-10-07)


### Bug Fixes

* revert serde_bytes to bytes with serde ([2e02460](https://github.com/sevki/jetstream/commit/2e02460c71890a59af315f29897d4edbe79f9c54))

## [5.2.1](https://github.com/sevki/jetstream/compare/v5.2.0...v5.2.1) (2024-10-06)


### Bug Fixes

* **wireformat:** from_bytes doesn't require a mutable buf ([437c35c](https://github.com/sevki/jetstream/commit/437c35c2229cd5980f752573fda43c1d6dfae725))

## [5.2.0](https://github.com/sevki/jetstream/compare/v5.1.3...v5.2.0) (2024-10-06)


### Features

* use serde_bytes::ByteBuf instead of Bytes ([a1101d9](https://github.com/sevki/jetstream/commit/a1101d99fcfc60ddff2314cebd99a56da706cf7b))


### Bug Fixes

* lint errors ([4f50d0b](https://github.com/sevki/jetstream/commit/4f50d0b73ddcfeaee2e86a2d27d061d87a0c1134))

## [5.1.4](https://github.com/sevki/jetstream/compare/v5.1.3...v5.1.4) (2024-10-03)


### Bug Fixes

* lint errors ([4f50d0b](https://github.com/sevki/jetstream/commit/4f50d0b73ddcfeaee2e86a2d27d061d87a0c1134))

## [5.1.3](https://github.com/sevki/jetstream/compare/v5.1.2...v5.1.3) (2024-10-03)


### Bug Fixes

* Update release.yml ([b058b38](https://github.com/sevki/jetstream/commit/b058b38367db97630b91af82e6bb11e26728708e))

## [5.1.2](https://github.com/sevki/jetstream/compare/v5.1.1...v5.1.2) (2024-10-03)


### Bug Fixes

* Update release.yml ([#122](https://github.com/sevki/jetstream/issues/122)) ([566fe1f](https://github.com/sevki/jetstream/commit/566fe1f64c6d6f25d157a227b261c69c9b3e83b5))

## [5.1.1](https://github.com/sevki/jetstream/compare/v5.1.0...v5.1.1) (2024-10-03)


### Bug Fixes

* release again ([3a6e65e](https://github.com/sevki/jetstream/commit/3a6e65e2d8181bcc93a41a577e2ca04c9ca70bf2))
* warnings ([62d8013](https://github.com/sevki/jetstream/commit/62d8013e3b9fa2c0120cd9269894ed640c03fb83))

## [5.1.0](https://github.com/sevki/jetstream/compare/v5.0.0...v5.1.0) (2024-10-03)


### Features

* **wireformat:** add u128 ([c76f6c4](https://github.com/sevki/jetstream/commit/c76f6c4c64fe57dd5948e522c8d68b8114b607ab))


### Bug Fixes

* release workflow ([4abeb24](https://github.com/sevki/jetstream/commit/4abeb24e99aec6ae9ddcc083136cc765b84ba094))

## [5.0.0](https://github.com/sevki/jetstream/compare/v4.0.0...v5.0.0) (2024-10-03)


### ⚠ BREAKING CHANGES

* splits up packages

### Features

* modularize components ([7262a66](https://github.com/sevki/jetstream/commit/7262a6665993d0d7705191717d773fadcac5173a))


### Bug Fixes

* version ([822bf0e](https://github.com/sevki/jetstream/commit/822bf0ed14786912a6ca9a7329a577f6d2367945))

## [4.0.0](https://github.com/sevki/jetstream/compare/v3.0.0...v4.0.0) (2024-10-01)


### ⚠ BREAKING CHANGES

* move modules to sensible parents

### Code Refactoring

* move modules to sensible parents ([4eba5fb](https://github.com/sevki/jetstream/commit/4eba5fb0105626fd56c42f2062c7cfbe7293279b))

## [3.0.0](https://github.com/sevki/jetstream/compare/v2.0.2...v3.0.0) (2024-03-30)


### ⚠ BREAKING CHANGES

* protocol -> coding
* merge all the creates

### Features

* autopub ([73a0844](https://github.com/sevki/jetstream/commit/73a0844e9a7fcc55bf39b39325587d237c549a6e))
* hide filesystem behind a feautre-flag ([9aa880d](https://github.com/sevki/jetstream/commit/9aa880de8d51c88e64d08248f47ddf1d0137db98))
* **macros:** service macro to remove boilerplate code ([e0a9295](https://github.com/sevki/jetstream/commit/e0a9295674327b5eea96922c3054d0e3be07c4a4))
* release please ([7d7bedd](https://github.com/sevki/jetstream/commit/7d7beddcae75613433076a9f77156989b2de1f47))
* release please ([044cceb](https://github.com/sevki/jetstream/commit/044cceb76e544e8c315b6e1d33a321795280e847))
* rust-clippy code scanning ([3dfb39f](https://github.com/sevki/jetstream/commit/3dfb39f1c4c9c887931a1686a7c1208fa1182e18))
* virtio support ([ce13217](https://github.com/sevki/jetstream/commit/ce13217e4429270226ef43661acab21619493351))


### Bug Fixes

* auto-release feature ([6505b0f](https://github.com/sevki/jetstream/commit/6505b0ff66ce16b1032efe722620d03fbe945769))
* bothced update ([b3b7003](https://github.com/sevki/jetstream/commit/b3b7003f565fc833804a70519aeb0741d03f34be))
* broken release-please ([089bb22](https://github.com/sevki/jetstream/commit/089bb2277ba7025b61dbff32473d9b4b1836acd9))
* ci release-please ([de391e5](https://github.com/sevki/jetstream/commit/de391e58d30f5f08d89f2f9251e9f734bd945bb1))
* filesystem under feature flag, rm newline ([de4cf79](https://github.com/sevki/jetstream/commit/de4cf791a89b794dbfcb48325b1a90f26e421616))
* ignore e2e tests ([e066dde](https://github.com/sevki/jetstream/commit/e066dde3f735c2524118ee0e8128555775d0eeb7))
* **macros:** protocol macro fix to spread fn args ([b261a28](https://github.com/sevki/jetstream/commit/b261a286e033e2f9ba1462dc8a5dd06adf4e5ca3))
* make data io::Read ([12a864e](https://github.com/sevki/jetstream/commit/12a864e93e402c0145ff1f206589164bc920fbdd))
* make data io::Read ([910c75a](https://github.com/sevki/jetstream/commit/910c75ad120d6b699691dc194377138837b2a8f5))
* make data io::Read ([77b3680](https://github.com/sevki/jetstream/commit/77b3680ff8ac312b14303f864d92bb763659b64f))
* Update client_tests.rs ([4c50132](https://github.com/sevki/jetstream/commit/4c50132ba0d6afa46f849db7d0fa356e64947653))
* update to v2 upload sarif ([e38bacb](https://github.com/sevki/jetstream/commit/e38bacb216db9b6560b3fe0e284b54425b4e251e))


### Code Refactoring

* merge all the creates ([faa0a1a](https://github.com/sevki/jetstream/commit/faa0a1a1194bac41d8e05efd0108e0c1821fa543))
* protocol -&gt; coding ([5f86bc7](https://github.com/sevki/jetstream/commit/5f86bc78a85728091f8411ab00f5ad3e4a960df2))

## [2.0.2](https://github.com/sevki/jetstream/compare/v2.0.1...v2.0.2) (2024-03-30)


### Bug Fixes

* make data io::Read ([12a864e](https://github.com/sevki/jetstream/commit/12a864e93e402c0145ff1f206589164bc920fbdd))
* make data io::Read ([910c75a](https://github.com/sevki/jetstream/commit/910c75ad120d6b699691dc194377138837b2a8f5))
* make data io::Read ([77b3680](https://github.com/sevki/jetstream/commit/77b3680ff8ac312b14303f864d92bb763659b64f))

## [2.0.1](https://github.com/sevki/jetstream/compare/v2.0.0...v2.0.1) (2024-03-29)


### Bug Fixes

* **macros:** protocol macro fix to spread fn args ([b261a28](https://github.com/sevki/jetstream/commit/b261a286e033e2f9ba1462dc8a5dd06adf4e5ca3))

## [2.0.0](https://github.com/sevki/jetstream/compare/v1.1.1...v2.0.0) (2024-03-29)


### ⚠ BREAKING CHANGES

* protocol -> coding

### Code Refactoring

* protocol -&gt; coding ([5f86bc7](https://github.com/sevki/jetstream/commit/5f86bc78a85728091f8411ab00f5ad3e4a960df2))

## [1.1.1](https://github.com/sevki/jetstream/compare/v1.1.0...v1.1.1) (2024-03-29)


### Bug Fixes

* ignore e2e tests ([e066dde](https://github.com/sevki/jetstream/commit/e066dde3f735c2524118ee0e8128555775d0eeb7))

## [1.1.0](https://github.com/sevki/jetstream/compare/v1.0.0...v1.1.0) (2024-03-29)


### Features

* **macros:** service macro to remove boilerplate code ([e0a9295](https://github.com/sevki/jetstream/commit/e0a9295674327b5eea96922c3054d0e3be07c4a4))

## [1.0.0](https://github.com/sevki/jetstream/compare/v0.6.0...v1.0.0) (2024-03-25)


### ⚠ BREAKING CHANGES

* merge all the creates

### Code Refactoring

* merge all the creates ([faa0a1a](https://github.com/sevki/jetstream/commit/faa0a1a1194bac41d8e05efd0108e0c1821fa543))

## [0.6.0](https://github.com/sevki/jetstream/compare/v0.5.1...v0.6.0) (2024-03-21)


### Features

* virtio support ([ce13217](https://github.com/sevki/jetstream/commit/ce13217e4429270226ef43661acab21619493351))

## [0.5.1](https://github.com/sevki/jetstream/compare/v0.5.0...v0.5.1) (2024-03-15)


### Bug Fixes

* filesystem under feature flag, rm newline ([de4cf79](https://github.com/sevki/jetstream/commit/de4cf791a89b794dbfcb48325b1a90f26e421616))
* Update client_tests.rs ([4c50132](https://github.com/sevki/jetstream/commit/4c50132ba0d6afa46f849db7d0fa356e64947653))

## [0.5.0](https://github.com/sevki/jetstream/compare/v0.4.0...v0.5.0) (2024-03-15)


### Features

* rust-clippy code scanning ([3dfb39f](https://github.com/sevki/jetstream/commit/3dfb39f1c4c9c887931a1686a7c1208fa1182e18))


### Bug Fixes

* update to v2 upload sarif ([e38bacb](https://github.com/sevki/jetstream/commit/e38bacb216db9b6560b3fe0e284b54425b4e251e))

## [0.4.0](https://github.com/sevki/jetstream/compare/v0.3.2...v0.4.0) (2024-03-14)


### Features

* autopub ([73a0844](https://github.com/sevki/jetstream/commit/73a0844e9a7fcc55bf39b39325587d237c549a6e))
* hide filesystem behind a feautre-flag ([9aa880d](https://github.com/sevki/jetstream/commit/9aa880de8d51c88e64d08248f47ddf1d0137db98))

## [0.3.2](https://github.com/sevki/jetstream/compare/v0.3.1...v0.3.2) (2024-03-14)


### Bug Fixes

* auto-release feature ([6505b0f](https://github.com/sevki/jetstream/commit/6505b0ff66ce16b1032efe722620d03fbe945769))

## [0.3.1](https://github.com/sevki/jetstream/compare/v0.3.0...v0.3.1) (2024-03-14)


### Bug Fixes

* broken release-please ([089bb22](https://github.com/sevki/jetstream/commit/089bb2277ba7025b61dbff32473d9b4b1836acd9))
* ci release-please ([de391e5](https://github.com/sevki/jetstream/commit/de391e58d30f5f08d89f2f9251e9f734bd945bb1))

## [0.3.0](https://github.com/sevki/jetstream/compare/v0.2.0...v0.3.0) (2024-03-14)


### Features

* release please ([7d7bedd](https://github.com/sevki/jetstream/commit/7d7beddcae75613433076a9f77156989b2de1f47))
* release please ([044cceb](https://github.com/sevki/jetstream/commit/044cceb76e544e8c315b6e1d33a321795280e847))

## [0.2.0](https://github.com/sevki/jetstream/compare/v0.1.4...v0.2.0) (2024-03-14)


### Features

* release please ([044cceb](https://github.com/sevki/jetstream/commit/044cceb76e544e8c315b6e1d33a321795280e847))
