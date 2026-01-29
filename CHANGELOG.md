# Changelog


## [14.0.0](https://github.com/sevki/jetstream/compare/v13.0.0...v14.0.0) (2026-01-29)


### ⚠ BREAKING CHANGES

* fully implenent jetstream_quic, introduced related traits

### Features

* fully implenent jetstream_quic, introduced related traits ([2b1e4a6](https://github.com/sevki/jetstream/commit/2b1e4a6fb4fdbfbf0d4247997c79b3300807350d))
* introduce jetstream_http, supports http1.1,http2 and h3 ([10c203b](https://github.com/sevki/jetstream/commit/10c203b8d76f8a6b17045576fd5107dd746ca86b))

## [13.0.0](https://github.com/sevki/jetstream/compare/v12.1.0...v13.0.0) (2026-01-25)


### ⚠ BREAKING CHANGES

* add multiplexing ability to client.
* make error types more isomorphic
* move quic and webocket to their own crates.
* use more futures
* fix proken publish
* splits up packages
* move modules to sensible parents
* protocol -> coding
* merge all the creates

### Features

* add cleanup code serverside. add cloudflare docs to mdbook ([91da99c](https://github.com/sevki/jetstream/commit/91da99cae4a47dbd80672574e75098909bbcb79b))
* add context ([4771c84](https://github.com/sevki/jetstream/commit/4771c84d8a7f3a35ff1b8c25ebe0b0871dd7592d))
* add i16,i32,i64,i128 types ([ba741bd](https://github.com/sevki/jetstream/commit/ba741bd0d2aed8d33d2b5589ca2a3da83b1abce8))
* add i16,i32,i64,i128 types ([3f0751a](https://github.com/sevki/jetstream/commit/3f0751a8588bf0fb5d189865f65417a717f414c4))
* add iroh docs ([6b7392f](https://github.com/sevki/jetstream/commit/6b7392f3c719e292a370c7c51e00506e49fb26b6))
* add multiple tag pool backends ([c85aafb](https://github.com/sevki/jetstream/commit/c85aafbb6606c9b89937cf1c3d6d1e2149fa3eae))
* add multiplexing ability to client. ([91a238e](https://github.com/sevki/jetstream/commit/91a238e30eff6b20a8d2b2f8bbce67e4bc865068))
* add prost_wireformat macro ([beeb947](https://github.com/sevki/jetstream/commit/beeb94747f66ba01ce98dec24bff83fda88f1a99))
* Add support for bool in WireFormat ([640c6ca](https://github.com/sevki/jetstream/commit/640c6ca3e22d0ed1dc7ec9c58af998e96efab235))
* Add tests for jetstream modules ([18462d9](https://github.com/sevki/jetstream/commit/18462d96f8dd71b528d49441dc257d2038f3e39f))
* add tracing feature ([5cb2907](https://github.com/sevki/jetstream/commit/5cb290799dd251c6e04ab4c2fe52089e3f32336c))
* add websocket transport ([3f4054c](https://github.com/sevki/jetstream/commit/3f4054c9ebe2253c11a2096c4983c7d75073da9a))
* add wireformat for SystemTime ([169f74a](https://github.com/sevki/jetstream/commit/169f74a162d477177ee81b07bb0315372644f2fd))
* added async_trait support service macro ([9a86185](https://github.com/sevki/jetstream/commit/9a86185b07eb02cd05933920f9ad02792992c71c))
* autopub ([73a0844](https://github.com/sevki/jetstream/commit/73a0844e9a7fcc55bf39b39325587d237c549a6e))
* better error handling ([faae10a](https://github.com/sevki/jetstream/commit/faae10a704821d723be55e0e6d92e88def17591c))
* channels can now be split ([ef646a5](https://github.com/sevki/jetstream/commit/ef646a590a5af13adc2c97c4871f19bca2a1d6b6))
* distributes jetsream ([d1477d5](https://github.com/sevki/jetstream/commit/d1477d56e574ca4c76b967a27c84ce7ccc4d9396))
* dt features, remove rustdoc_to_md_book ([0398159](https://github.com/sevki/jetstream/commit/039815903e127055ec494a3dc3e1780b7afb4fe2))
* enum support ([c4552d8](https://github.com/sevki/jetstream/commit/c4552d8bd056debdbb7845145f964defae6f90d2))
* fix proken publish ([a7272c0](https://github.com/sevki/jetstream/commit/a7272c0f3dc25aa10e89c8b7e9ad070d919a369d))
* hide filesystem behind a feautre-flag ([9aa880d](https://github.com/sevki/jetstream/commit/9aa880de8d51c88e64d08248f47ddf1d0137db98))
* introduce iroh examples ([86e001f](https://github.com/sevki/jetstream/commit/86e001f9195a28429e34670597347d3148ea22d9))
* introduce jetstream cloudflare module. ([6e8cc01](https://github.com/sevki/jetstream/commit/6e8cc015024d3066f9f3208e20bd4725c03b0ac2))
* Ip primitives ([1c263b5](https://github.com/sevki/jetstream/commit/1c263b59494e2699ec2347727f6cbf77bdac3b67))
* jetstream cluster ([80b5727](https://github.com/sevki/jetstream/commit/80b5727722b1b44d522eb793c18e5bf117660d8b))
* jetstream distributed features ([52b5e89](https://github.com/sevki/jetstream/commit/52b5e89e64b1e5559e600bed5fd91499cd051934))
* **jetstream_distributed:** placement ([8a93788](https://github.com/sevki/jetstream/commit/8a93788e51a98807466aa868b13685ba81aa1d3e))
* jetstream_libc ([aba123b](https://github.com/sevki/jetstream/commit/aba123bb7a027a97cad4d8a9d7731d18bb4059ca))
* jetstream_rpc supports wasm ([e97e6ca](https://github.com/sevki/jetstream/commit/e97e6ca6a94067551fb7a98c4e34fa2a774ebdfd))
* **macros:** service macro to remove boilerplate code ([e0a9295](https://github.com/sevki/jetstream/commit/e0a9295674327b5eea96922c3054d0e3be07c4a4))
* modularize components ([7262a66](https://github.com/sevki/jetstream/commit/7262a6665993d0d7705191717d773fadcac5173a))
* move quic and webocket to their own crates. ([7d0ba9f](https://github.com/sevki/jetstream/commit/7d0ba9fae68f1e4995d9ea6d1adcaf95b8dc983a))
* **prost_wireformat:** make derives optional ([12dfacb](https://github.com/sevki/jetstream/commit/12dfacbc0eb6287d90e50ed8ec2dcd11cf41d24b))
* publish npm package to gh ([9b332b9](https://github.com/sevki/jetstream/commit/9b332b9f5a26a73ce72ec07e3e4e6315cc1d47c1))
* release please ([5fe2180](https://github.com/sevki/jetstream/commit/5fe2180cddc74c5933bac93c96d35a3fbd8653de))
* release please ([7d7bedd](https://github.com/sevki/jetstream/commit/7d7beddcae75613433076a9f77156989b2de1f47))
* revamp service ([#147](https://github.com/sevki/jetstream/issues/147)) ([6d96be8](https://github.com/sevki/jetstream/commit/6d96be89affc75b9e534122ac9305257861be706))
* rust-clippy code scanning ([3dfb39f](https://github.com/sevki/jetstream/commit/3dfb39f1c4c9c887931a1686a7c1208fa1182e18))
* rustdoc_to_mdbook ([32deadf](https://github.com/sevki/jetstream/commit/32deadf98614a9365e9991c3cd76620280c85b5d))
* start quic server with config ([#159](https://github.com/sevki/jetstream/issues/159)) ([3433981](https://github.com/sevki/jetstream/commit/34339819f26044e0a19c5c668c2fb710734af564))
* update mdbook ([f213c71](https://github.com/sevki/jetstream/commit/f213c712bcc5395b048f83b2c352c163c11518e0))
* use more futures ([467b6f5](https://github.com/sevki/jetstream/commit/467b6f56d0808e89f078c656f1e7c447738244ed))
* use sccache ([#142](https://github.com/sevki/jetstream/issues/142)) ([89f96ab](https://github.com/sevki/jetstream/commit/89f96abf5b0527d45bf75e00fafd4cd197fff1d6))
* use serde_bytes::ByteBuf instead of Bytes ([a1101d9](https://github.com/sevki/jetstream/commit/a1101d99fcfc60ddff2314cebd99a56da706cf7b))
* use tag pool in mux ([cd5dd5d](https://github.com/sevki/jetstream/commit/cd5dd5db7be347b586becd64e3113c5f34d3bd09))
* virtio support ([ce13217](https://github.com/sevki/jetstream/commit/ce13217e4429270226ef43661acab21619493351))
* wasm-support ([#260](https://github.com/sevki/jetstream/issues/260)) ([5cbff0d](https://github.com/sevki/jetstream/commit/5cbff0daf95c780cd5962252aaa975d6248b524b))
* **wireformat:** add u128 ([c76f6c4](https://github.com/sevki/jetstream/commit/c76f6c4c64fe57dd5948e522c8d68b8114b607ab))


### Bug Fixes

* add a script to update the workspace versions ([9fc4ca2](https://github.com/sevki/jetstream/commit/9fc4ca29c2e3d99c37d10c75bc439540f26d7cea))
* add comment about cf exec model ([c979856](https://github.com/sevki/jetstream/commit/c979856a7406e0f612fc26adbfa200471034fbeb))
* add support for libc in windows and mac ([1fb3b5b](https://github.com/sevki/jetstream/commit/1fb3b5ba61c36372db24231f66753453a05b149a))
* add toasty types fix skip ([0fe936a](https://github.com/sevki/jetstream/commit/0fe936abdda512235343a77ff29fed7c972d3f96))
* auto-release ([964036c](https://github.com/sevki/jetstream/commit/964036cd97bfbae055f3b52653387d65524f1972))
* auto-release feature ([6505b0f](https://github.com/sevki/jetstream/commit/6505b0ff66ce16b1032efe722620d03fbe945769))
* benchmarks ([af695e2](https://github.com/sevki/jetstream/commit/af695e28faa054b8434a13ae963d2fc6abab5d3a))
* benchmarks ([94767dd](https://github.com/sevki/jetstream/commit/94767ddfbd2b43f9562621dbbd679b3ec2215da6))
* benchmarks and add iroh_benchmarks ([56dbc12](https://github.com/sevki/jetstream/commit/56dbc12c0938ed5aa1b22d8e4867beaaf7b0d587))
* benchmarks code ([c0d0f59](https://github.com/sevki/jetstream/commit/c0d0f5948757ef9a7e6a0a803a51163710154ba1))
* bothced update ([b3b7003](https://github.com/sevki/jetstream/commit/b3b7003f565fc833804a70519aeb0741d03f34be))
* broken lock file ([6148cf7](https://github.com/sevki/jetstream/commit/6148cf776ac29ab9941574422aeda44ed01450ee))
* broken okid lockfile ([63bae5e](https://github.com/sevki/jetstream/commit/63bae5e68e8707273b13dd06a74981f635556923))
* broken release-please ([089bb22](https://github.com/sevki/jetstream/commit/089bb2277ba7025b61dbff32473d9b4b1836acd9))
* broken use statement for async_trait ([cce4df6](https://github.com/sevki/jetstream/commit/cce4df6a5a594f25fcf71eb63e51534bf8b85c3b))
* bump deps ([0dbd81b](https://github.com/sevki/jetstream/commit/0dbd81b248e3d34aee458116f7aa82cddc31d570))
* bump okid ([7ed2940](https://github.com/sevki/jetstream/commit/7ed29402ed7a2c41d2f88696cefec4a42c09ec38))
* bump zero copy ([#140](https://github.com/sevki/jetstream/issues/140)) ([4bb933f](https://github.com/sevki/jetstream/commit/4bb933f8ab6399e44e9a9ba4dc9ff92b43d69454))
* cargo lock resolver ([327d8bd](https://github.com/sevki/jetstream/commit/327d8bdfe9d11f3d4d99e245de100ffae9e5d94b))
* cargo publish workspace ([8db9367](https://github.com/sevki/jetstream/commit/8db93673748a9b5684f746ab55a34c289df70554))
* cargo toml version issues ([3481015](https://github.com/sevki/jetstream/commit/3481015e9fd2ffb3fc881168bb38ee03c72144d5))
* Change git user configuration in workflow ([c3d92c1](https://github.com/sevki/jetstream/commit/c3d92c19a876d5d433e15ce47cb5977b23eff7a9))
* change service shape ([194252d](https://github.com/sevki/jetstream/commit/194252db4e3a58509845692a09a77fd9d1ccac2f))
* ci ([e7418e5](https://github.com/sevki/jetstream/commit/e7418e584a5b45965666f85bbfbf9795fc1a5b7c))
* ci release-please ([de391e5](https://github.com/sevki/jetstream/commit/de391e58d30f5f08d89f2f9251e9f734bd945bb1))
* ci workflows ([4c12f04](https://github.com/sevki/jetstream/commit/4c12f046e9ff9a5d39f4f0619f6ed68b25edcfe7))
* ci-builds, drop 1.76 ([5664e57](https://github.com/sevki/jetstream/commit/5664e571b607eb8ed3b6f9ee3488994b4cd568b9))
* ci/cd ([#157](https://github.com/sevki/jetstream/issues/157)) ([3f7ff1e](https://github.com/sevki/jetstream/commit/3f7ff1ea0b561136bc51c65f1acc0f683cfafb01))
* cleanup code and remove unused mermaid ([c8975d5](https://github.com/sevki/jetstream/commit/c8975d56c388470fb4305c569ee30b84fde30f81))
* collections can only be u16::MAX ([6ada71a](https://github.com/sevki/jetstream/commit/6ada71a7222d16b0e3035837c76b8e3bcb51f627))
* correct release workflow step ordering ([0c7bbcd](https://github.com/sevki/jetstream/commit/0c7bbcd0403718dd0f18432c699ee9defc54068b))
* criterion benchmark test, framed implementation ([25b1611](https://github.com/sevki/jetstream/commit/25b1611be97ffde52a1cb403ea4a55e3aa5f8e0b))
* Delete CHANGELOG.md ([abcc79d](https://github.com/sevki/jetstream/commit/abcc79dc0511230f4725ea5da20c624352635b65))
* delete release-plz ([a7e3199](https://github.com/sevki/jetstream/commit/a7e319947a95b0915d01896688a13c86b77fc440))
* dependency order ([9a0e1f9](https://github.com/sevki/jetstream/commit/9a0e1f979c85a45780bf71f669da4cc91ecd3f5b))
* dependency order and cycle ([6a6c997](https://github.com/sevki/jetstream/commit/6a6c99778f90f8481d5792d560a6e97e0c5926bc))
* dependency sudoku ([7df5f9b](https://github.com/sevki/jetstream/commit/7df5f9bdce4a3a830609fe5f50bfc3cdd278b5d2))
* disable running echo on windows ([828d6be](https://github.com/sevki/jetstream/commit/828d6be26328a938417b976cb8c8f63ddc1a9bea))
* disable running echo on windows ([7f44956](https://github.com/sevki/jetstream/commit/7f44956f965d5c906812e91b138e5c21597e37df))
* docs ([995da2f](https://github.com/sevki/jetstream/commit/995da2f79e1fc42eeb8a87811d070094b6e9e4b1))
* elide lifetimes in ufs ([b778ff9](https://github.com/sevki/jetstream/commit/b778ff9481bf9c9d1cf35fc691903b80483e8cdb))
* extern async_trait and trait_variant and lazy_static ([71e93e2](https://github.com/sevki/jetstream/commit/71e93e2db32236e01bfc2cc923a6db10a788a730))
* extern_more ([1bd58d8](https://github.com/sevki/jetstream/commit/1bd58d8150031c1394f94e7074afd2c1c60928de))
* extract git config to separate step ([679d1a7](https://github.com/sevki/jetstream/commit/679d1a75fbe52ec995c01ad95a53afce994bc8fe))
* failing docs ([232ddee](https://github.com/sevki/jetstream/commit/232ddee45d616dc08ea7e6dcd072db2c7f2cbba6))
* failing serde tests ([81db0de](https://github.com/sevki/jetstream/commit/81db0ded08429489cf5d83d5540804b00fdb9802))
* filesystem under feature flag, rm newline ([de4cf79](https://github.com/sevki/jetstream/commit/de4cf791a89b794dbfcb48325b1a90f26e421616))
* formatting ([049e584](https://github.com/sevki/jetstream/commit/049e58478f81be41983bdeef4a661f12eb21215a))
* formatting ([04aace4](https://github.com/sevki/jetstream/commit/04aace42d2d1338e614081954e32448ac3e5ec30))
* fuzz target ([a494e15](https://github.com/sevki/jetstream/commit/a494e15fca5b0ee1e66399d2b50cf0e1158b070c))
* hide documentation for iroh ([47b104d](https://github.com/sevki/jetstream/commit/47b104d73ece751924c1e41a77a92a8317c968f9))
* ignore e2e tests ([e066dde](https://github.com/sevki/jetstream/commit/e066dde3f735c2524118ee0e8128555775d0eeb7))
* iroh example needs -F iroh ([5246fb2](https://github.com/sevki/jetstream/commit/5246fb234453964ebf0e0f12ad0437f23780032d))
* keep reciver types as is in generated code ([7d95671](https://github.com/sevki/jetstream/commit/7d956718c4e4c1564b1aeeb2d6af635eb44ae220))
* lint errors ([4f50d0b](https://github.com/sevki/jetstream/commit/4f50d0b73ddcfeaee2e86a2d27d061d87a0c1134))
* lint errors and rm unnecessary compiler_error ([27aed1e](https://github.com/sevki/jetstream/commit/27aed1ef8263e4750f6a19dcd971d5b438dd1668))
* **macros:** protocol macro fix to spread fn args ([b261a28](https://github.com/sevki/jetstream/commit/b261a286e033e2f9ba1462dc8a5dd06adf4e5ca3))
* make data io::Read ([12a864e](https://github.com/sevki/jetstream/commit/12a864e93e402c0145ff1f206589164bc920fbdd))
* make data io::Read ([910c75a](https://github.com/sevki/jetstream/commit/910c75ad120d6b699691dc194377138837b2a8f5))
* make data io::Read ([77b3680](https://github.com/sevki/jetstream/commit/77b3680ff8ac312b14303f864d92bb763659b64f))
* make error types more isomorphic ([aee2fa2](https://github.com/sevki/jetstream/commit/aee2fa21a7c6879f5d77765d3ff96b22b98e0e86))
* make qid eq,hash ([522de0d](https://github.com/sevki/jetstream/commit/522de0d41e5cbda8d29ea6a20f1cb2cba7b5bb82))
* make TagPool use a channel ([beb6209](https://github.com/sevki/jetstream/commit/beb6209632a2a8abe283387eea531d8728c55ce2))
* make the proto mod same vis as trait ([a43c0a2](https://github.com/sevki/jetstream/commit/a43c0a2e3fbcb2991f6f3396eba9e30c96a47663))
* make websocket pub ([6096ff9](https://github.com/sevki/jetstream/commit/6096ff9815b3b15dac1cfdd124821dc9d40218b0))
* make websocket pub again ([21acf4b](https://github.com/sevki/jetstream/commit/21acf4bd6f9269ff9c0ce3b4c3654bbfa2ca6d98))
* mdbook changelog ([5f03b6b](https://github.com/sevki/jetstream/commit/5f03b6b7e2fe7b2fb56f082f8ff56105066b0998))
* mdbook-changelog version issue ([7043b8e](https://github.com/sevki/jetstream/commit/7043b8e1b3ef85b1da383e6e2488bc32e4c9f839))
* more tests ([9a5071f](https://github.com/sevki/jetstream/commit/9a5071fb22c4a63e0af4fb1af9c7133f00cbc26e))
* move term-transcript to dev, gate s2n windows ([5fa7f4c](https://github.com/sevki/jetstream/commit/5fa7f4ce55ef111c0a155416d556189fbc266b87))
* npm scoping issue ([0d06884](https://github.com/sevki/jetstream/commit/0d0688482a692a7766e1dca5764ff4189532cd59))
* only build docs in release ([02b02bc](https://github.com/sevki/jetstream/commit/02b02bc6348a3ec537683d9b18047c02e8b7c502))
* option&lt;T&gt; support ([2e224ca](https://github.com/sevki/jetstream/commit/2e224cad47d775fb2c967c4d37d33b8aabeb7470))
* pin toasty ([03b6b18](https://github.com/sevki/jetstream/commit/03b6b18c6cce3cf9cdb274040bf29da21e1e5dd2))
* publish script ([e8685cc](https://github.com/sevki/jetstream/commit/e8685cc195682d2467de6d042ff3052ddd9175c5))
* readme ([dc77722](https://github.com/sevki/jetstream/commit/dc777223fe95ea79866320b7831e4e3c89631820))
* recursive self call ([c51d455](https://github.com/sevki/jetstream/commit/c51d455ececd5ac461561b8bd44709a92a45b2b7))
* reenable sccache ([757bb7e](https://github.com/sevki/jetstream/commit/757bb7e248996a587ee13c2a8c21246d0e198a04))
* reexport tokio_util from rpc ([cf28c40](https://github.com/sevki/jetstream/commit/cf28c40a3783379ac89295fef4a55bd4212d4891))
* release again ([3a6e65e](https://github.com/sevki/jetstream/commit/3a6e65e2d8181bcc93a41a577e2ca04c9ca70bf2))
* release order ([0973046](https://github.com/sevki/jetstream/commit/0973046516c6703f7f3fb97d8732d4130ffaa729))
* release please ([53a698b](https://github.com/sevki/jetstream/commit/53a698b53ad31d12cc068ba8e921779b710a448e))
* release please ([92eeee5](https://github.com/sevki/jetstream/commit/92eeee530c2be541a31a46d86dcd67fd68b0c8a5))
* release please ([3a2f4df](https://github.com/sevki/jetstream/commit/3a2f4df4bbec17361295a2839ba10e521656f94f))
* release workflow ([4abeb24](https://github.com/sevki/jetstream/commit/4abeb24e99aec6ae9ddcc083136cc765b84ba094))
* release-please-config.json ([885527f](https://github.com/sevki/jetstream/commit/885527f075deffba2f139739032460e35d93bc08))
* remove distributed ([3aba1f6](https://github.com/sevki/jetstream/commit/3aba1f6543493e107b079d35fa441d46611e637e))
* remove expects ([8a58cf5](https://github.com/sevki/jetstream/commit/8a58cf565453d755100b5298c1c89ed478dff11e))
* remove jj ([02fefd9](https://github.com/sevki/jetstream/commit/02fefd9e82f0f260d54f703daa0eb4667cdbce5d))
* remove prost ([5a5ff9b](https://github.com/sevki/jetstream/commit/5a5ff9b7904f1023f5bbed6c5a6236ef01644c17))
* remove redundant io imports and use fully qualified paths ([7de4b9e](https://github.com/sevki/jetstream/commit/7de4b9ea0cece01326cde4af027eae8a3120f1f9))
* remove redundant push from version update step ([923d696](https://github.com/sevki/jetstream/commit/923d6969e1435d261a16bbe2e156be834304ed0f))
* remove tracing from sink and stream ([97ac0a5](https://github.com/sevki/jetstream/commit/97ac0a59b3c0cd8aefb2b0af2804b81316ea6acf))
* remove unsafe code ([a286d93](https://github.com/sevki/jetstream/commit/a286d93fddc7bf8329defe77b5d01c0b6dbabd17))
* revert serde_bytes to bytes with serde ([2e02460](https://github.com/sevki/jetstream/commit/2e02460c71890a59af315f29897d4edbe79f9c54))
* rollback toasty ([ae0790b](https://github.com/sevki/jetstream/commit/ae0790b473fa81e1c67ec25003bec0f0d6082450))
* rust.yml workflow ([9dc7bc0](https://github.com/sevki/jetstream/commit/9dc7bc09ad8a341e8bcf19f945b8efa478b0d67d))
* rustdoc to mdbook ([2098ad1](https://github.com/sevki/jetstream/commit/2098ad11072e31ec329b42ae7df6dd380d993ff7))
* semaphor should use permit.forget() ([340323c](https://github.com/sevki/jetstream/commit/340323c9039fce8b6d60cf39d9e7ceca2b1b322e))
* service macro works correctly with client calls ([eb1fd0f](https://github.com/sevki/jetstream/commit/eb1fd0f01bc899b7f0650dbfca29e2d77ec76720))
* simpler workflows for github actions ([17185dd](https://github.com/sevki/jetstream/commit/17185ddd3337fff27131f96a26eabcb0d1961155))
* snapshots for macros ([fab686d](https://github.com/sevki/jetstream/commit/fab686df8c79710b5e9319519d8aa3d17b203847))
* some tests ([c65ded1](https://github.com/sevki/jetstream/commit/c65ded107a254d283e165563b0b97fa735741399))
* switch to release-plz ([a07aaec](https://github.com/sevki/jetstream/commit/a07aaec0dd9a38be953b739ad9269650d690d885))
* target os cfg attrs ([16e5c84](https://github.com/sevki/jetstream/commit/16e5c849b2c1b7b962590dfab779f4429cf06445))
* trait_variant use ([#155](https://github.com/sevki/jetstream/issues/155)) ([3cd5665](https://github.com/sevki/jetstream/commit/3cd5665454bb42c3e38606c4a3dd9355ef63fc9b))
* typo ([901ffbd](https://github.com/sevki/jetstream/commit/901ffbdda65336060410f348fe57c0ddc01a9fc4))
* unused git ([3fe908e](https://github.com/sevki/jetstream/commit/3fe908e405bc870a8089b18e88dae245f41eeb68))
* update 9p to use trait_variant ([96db410](https://github.com/sevki/jetstream/commit/96db410d8e5e04c4beb9100be2b58c271cdd7b8c))
* Update CHANGELOG.md ([6ab562f](https://github.com/sevki/jetstream/commit/6ab562f5210fa30a0a33a393d4fa18f3eb1154db))
* Update client_tests.rs ([4c50132](https://github.com/sevki/jetstream/commit/4c50132ba0d6afa46f849db7d0fa356e64947653))
* update doc links ([80a5e1e](https://github.com/sevki/jetstream/commit/80a5e1efc9b2a275b8d49be7eedc832602914c0c))
* update docs ([74e2867](https://github.com/sevki/jetstream/commit/74e2867864e46b441ed01193220c22842fcf67a8))
* Update lib.rs for extern crates ([ffb6777](https://github.com/sevki/jetstream/commit/ffb677744364c3d44274be2d5eb20c30ecabb6b5))
* update release flow ([#144](https://github.com/sevki/jetstream/issues/144)) ([36dd4af](https://github.com/sevki/jetstream/commit/36dd4af7686d405eeaa42d53e7eaac934d901489))
* Update release-please-config.json ([e592cf3](https://github.com/sevki/jetstream/commit/e592cf3ed85bf657b26e792e149848b5623bbd5b))
* Update release.yml ([b058b38](https://github.com/sevki/jetstream/commit/b058b38367db97630b91af82e6bb11e26728708e))
* Update release.yml ([#122](https://github.com/sevki/jetstream/issues/122)) ([566fe1f](https://github.com/sevki/jetstream/commit/566fe1f64c6d6f25d157a227b261c69c9b3e83b5))
* update snapshot tests ([b9fde4c](https://github.com/sevki/jetstream/commit/b9fde4c2a2192d58c839d7d9e45ea0d2110089f4))
* update to v2 upload sarif ([e38bacb](https://github.com/sevki/jetstream/commit/e38bacb216db9b6560b3fe0e284b54425b4e251e))
* use cargo publish workspace ([e28a0f7](https://github.com/sevki/jetstream/commit/e28a0f75bfa9fca9a990c6f2b5527be5e25a3d16))
* version ([ca861a7](https://github.com/sevki/jetstream/commit/ca861a7ac9797925d208c3b08e8d195607d4d388))
* version ([822bf0e](https://github.com/sevki/jetstream/commit/822bf0ed14786912a6ca9a7329a577f6d2367945))
* versioning issues ([4a3111c](https://github.com/sevki/jetstream/commit/4a3111cc18be2856b0d4355a0da2ec3f011eebf9))
* versions in workspace ([8b90b5a](https://github.com/sevki/jetstream/commit/8b90b5af219a36fa4934ace0e6411baedf816843))
* warnings ([62d8013](https://github.com/sevki/jetstream/commit/62d8013e3b9fa2c0120cd9269894ed640c03fb83))
* wasm-pack build error. rm rustdoc2mdbook ([68076cc](https://github.com/sevki/jetstream/commit/68076ccbcf88d18d1dadb30355d8358283864b54))
* wasm32 feature gates ([755f9c1](https://github.com/sevki/jetstream/commit/755f9c1c6628fc46b1d4d3df03441106de3abdfd))
* **wireformat:** from_bytes doesn't require a mutable buf ([437c35c](https://github.com/sevki/jetstream/commit/437c35c2229cd5980f752573fda43c1d6dfae725))


### Code Refactoring

* merge all the creates ([faa0a1a](https://github.com/sevki/jetstream/commit/faa0a1a1194bac41d8e05efd0108e0c1821fa543))
* move modules to sensible parents ([4eba5fb](https://github.com/sevki/jetstream/commit/4eba5fb0105626fd56c42f2062c7cfbe7293279b))
* protocol -&gt; coding ([5f86bc7](https://github.com/sevki/jetstream/commit/5f86bc78a85728091f8411ab00f5ad3e4a960df2))

## [12.1.0](https://github.com/sevki/jetstream/compare/v12.0.0...v12.1.0) (2026-01-25)


### Features

* publish npm package to gh ([9b332b9](https://github.com/sevki/jetstream/commit/9b332b9f5a26a73ce72ec07e3e4e6315cc1d47c1))


### Bug Fixes

* Change git user configuration in workflow ([c3d92c1](https://github.com/sevki/jetstream/commit/c3d92c19a876d5d433e15ce47cb5977b23eff7a9))
* npm scoping issue ([0d06884](https://github.com/sevki/jetstream/commit/0d0688482a692a7766e1dca5764ff4189532cd59))

## [12.0.0](https://github.com/sevki/jetstream/compare/v11.1.0...v12.0.0) (2026-01-24)


### ⚠ BREAKING CHANGES

* add multiplexing ability to client.

### Features

* add multiple tag pool backends ([c85aafb](https://github.com/sevki/jetstream/commit/c85aafbb6606c9b89937cf1c3d6d1e2149fa3eae))
* add multiplexing ability to client. ([91a238e](https://github.com/sevki/jetstream/commit/91a238e30eff6b20a8d2b2f8bbce67e4bc865068))
* use tag pool in mux ([cd5dd5d](https://github.com/sevki/jetstream/commit/cd5dd5db7be347b586becd64e3113c5f34d3bd09))


### Bug Fixes

* benchmarks code ([c0d0f59](https://github.com/sevki/jetstream/commit/c0d0f5948757ef9a7e6a0a803a51163710154ba1))
* delete release-plz ([a7e3199](https://github.com/sevki/jetstream/commit/a7e319947a95b0915d01896688a13c86b77fc440))
* lint errors and rm unnecessary compiler_error ([27aed1e](https://github.com/sevki/jetstream/commit/27aed1ef8263e4750f6a19dcd971d5b438dd1668))
* make TagPool use a channel ([beb6209](https://github.com/sevki/jetstream/commit/beb6209632a2a8abe283387eea531d8728c55ce2))
* semaphor should use permit.forget() ([340323c](https://github.com/sevki/jetstream/commit/340323c9039fce8b6d60cf39d9e7ceca2b1b322e))

## [11.1.0](https://github.com/sevki/jetstream/compare/v11.0.0...v11.1.0) (2026-01-21)


### Features

* add prost_wireformat macro ([beeb947](https://github.com/sevki/jetstream/commit/beeb94747f66ba01ce98dec24bff83fda88f1a99))
* add wireformat for SystemTime ([169f74a](https://github.com/sevki/jetstream/commit/169f74a162d477177ee81b07bb0315372644f2fd))
* **prost_wireformat:** make derives optional ([12dfacb](https://github.com/sevki/jetstream/commit/12dfacbc0eb6287d90e50ed8ec2dcd11cf41d24b))

## [11.0.0](https://github.com/sevki/jetstream/compare/v10.1.0...v11.0.0) (2026-01-20)


### ⚠ BREAKING CHANGES

* make error types more isomorphic
* move quic and webocket to their own crates.
* use more futures
* fix proken publish
* splits up packages
* move modules to sensible parents
* protocol -> coding
* merge all the creates

### Features

* add cleanup code serverside. add cloudflare docs to mdbook ([91da99c](https://github.com/sevki/jetstream/commit/91da99cae4a47dbd80672574e75098909bbcb79b))
* add context ([4771c84](https://github.com/sevki/jetstream/commit/4771c84d8a7f3a35ff1b8c25ebe0b0871dd7592d))
* add i16,i32,i64,i128 types ([ba741bd](https://github.com/sevki/jetstream/commit/ba741bd0d2aed8d33d2b5589ca2a3da83b1abce8))
* add i16,i32,i64,i128 types ([3f0751a](https://github.com/sevki/jetstream/commit/3f0751a8588bf0fb5d189865f65417a717f414c4))
* add iroh docs ([6b7392f](https://github.com/sevki/jetstream/commit/6b7392f3c719e292a370c7c51e00506e49fb26b6))
* Add support for bool in WireFormat ([640c6ca](https://github.com/sevki/jetstream/commit/640c6ca3e22d0ed1dc7ec9c58af998e96efab235))
* Add tests for jetstream modules ([18462d9](https://github.com/sevki/jetstream/commit/18462d96f8dd71b528d49441dc257d2038f3e39f))
* add tracing feature ([5cb2907](https://github.com/sevki/jetstream/commit/5cb290799dd251c6e04ab4c2fe52089e3f32336c))
* add websocket transport ([3f4054c](https://github.com/sevki/jetstream/commit/3f4054c9ebe2253c11a2096c4983c7d75073da9a))
* added async_trait support service macro ([9a86185](https://github.com/sevki/jetstream/commit/9a86185b07eb02cd05933920f9ad02792992c71c))
* autopub ([73a0844](https://github.com/sevki/jetstream/commit/73a0844e9a7fcc55bf39b39325587d237c549a6e))
* better error handling ([faae10a](https://github.com/sevki/jetstream/commit/faae10a704821d723be55e0e6d92e88def17591c))
* channels can now be split ([ef646a5](https://github.com/sevki/jetstream/commit/ef646a590a5af13adc2c97c4871f19bca2a1d6b6))
* distributes jetsream ([d1477d5](https://github.com/sevki/jetstream/commit/d1477d56e574ca4c76b967a27c84ce7ccc4d9396))
* dt features, remove rustdoc_to_md_book ([0398159](https://github.com/sevki/jetstream/commit/039815903e127055ec494a3dc3e1780b7afb4fe2))
* enum support ([c4552d8](https://github.com/sevki/jetstream/commit/c4552d8bd056debdbb7845145f964defae6f90d2))
* fix proken publish ([a7272c0](https://github.com/sevki/jetstream/commit/a7272c0f3dc25aa10e89c8b7e9ad070d919a369d))
* hide filesystem behind a feautre-flag ([9aa880d](https://github.com/sevki/jetstream/commit/9aa880de8d51c88e64d08248f47ddf1d0137db98))
* introduce iroh examples ([86e001f](https://github.com/sevki/jetstream/commit/86e001f9195a28429e34670597347d3148ea22d9))
* introduce jetstream cloudflare module. ([6e8cc01](https://github.com/sevki/jetstream/commit/6e8cc015024d3066f9f3208e20bd4725c03b0ac2))
* Ip primitives ([1c263b5](https://github.com/sevki/jetstream/commit/1c263b59494e2699ec2347727f6cbf77bdac3b67))
* jetstream cluster ([80b5727](https://github.com/sevki/jetstream/commit/80b5727722b1b44d522eb793c18e5bf117660d8b))
* jetstream distributed features ([52b5e89](https://github.com/sevki/jetstream/commit/52b5e89e64b1e5559e600bed5fd91499cd051934))
* **jetstream_distributed:** placement ([8a93788](https://github.com/sevki/jetstream/commit/8a93788e51a98807466aa868b13685ba81aa1d3e))
* jetstream_libc ([aba123b](https://github.com/sevki/jetstream/commit/aba123bb7a027a97cad4d8a9d7731d18bb4059ca))
* jetstream_rpc supports wasm ([e97e6ca](https://github.com/sevki/jetstream/commit/e97e6ca6a94067551fb7a98c4e34fa2a774ebdfd))
* **macros:** service macro to remove boilerplate code ([e0a9295](https://github.com/sevki/jetstream/commit/e0a9295674327b5eea96922c3054d0e3be07c4a4))
* modularize components ([7262a66](https://github.com/sevki/jetstream/commit/7262a6665993d0d7705191717d773fadcac5173a))
* move quic and webocket to their own crates. ([7d0ba9f](https://github.com/sevki/jetstream/commit/7d0ba9fae68f1e4995d9ea6d1adcaf95b8dc983a))
* release please ([5fe2180](https://github.com/sevki/jetstream/commit/5fe2180cddc74c5933bac93c96d35a3fbd8653de))
* release please ([7d7bedd](https://github.com/sevki/jetstream/commit/7d7beddcae75613433076a9f77156989b2de1f47))
* release please ([044cceb](https://github.com/sevki/jetstream/commit/044cceb76e544e8c315b6e1d33a321795280e847))
* revamp service ([#147](https://github.com/sevki/jetstream/issues/147)) ([6d96be8](https://github.com/sevki/jetstream/commit/6d96be89affc75b9e534122ac9305257861be706))
* rust-clippy code scanning ([3dfb39f](https://github.com/sevki/jetstream/commit/3dfb39f1c4c9c887931a1686a7c1208fa1182e18))
* rustdoc_to_mdbook ([32deadf](https://github.com/sevki/jetstream/commit/32deadf98614a9365e9991c3cd76620280c85b5d))
* start quic server with config ([#159](https://github.com/sevki/jetstream/issues/159)) ([3433981](https://github.com/sevki/jetstream/commit/34339819f26044e0a19c5c668c2fb710734af564))
* update mdbook ([f213c71](https://github.com/sevki/jetstream/commit/f213c712bcc5395b048f83b2c352c163c11518e0))
* use more futures ([467b6f5](https://github.com/sevki/jetstream/commit/467b6f56d0808e89f078c656f1e7c447738244ed))
* use sccache ([#142](https://github.com/sevki/jetstream/issues/142)) ([89f96ab](https://github.com/sevki/jetstream/commit/89f96abf5b0527d45bf75e00fafd4cd197fff1d6))
* use serde_bytes::ByteBuf instead of Bytes ([a1101d9](https://github.com/sevki/jetstream/commit/a1101d99fcfc60ddff2314cebd99a56da706cf7b))
* virtio support ([ce13217](https://github.com/sevki/jetstream/commit/ce13217e4429270226ef43661acab21619493351))
* wasm-support ([#260](https://github.com/sevki/jetstream/issues/260)) ([5cbff0d](https://github.com/sevki/jetstream/commit/5cbff0daf95c780cd5962252aaa975d6248b524b))
* **wireformat:** add u128 ([c76f6c4](https://github.com/sevki/jetstream/commit/c76f6c4c64fe57dd5948e522c8d68b8114b607ab))


### Bug Fixes

* add a script to update the workspace versions ([9fc4ca2](https://github.com/sevki/jetstream/commit/9fc4ca29c2e3d99c37d10c75bc439540f26d7cea))
* add comment about cf exec model ([c979856](https://github.com/sevki/jetstream/commit/c979856a7406e0f612fc26adbfa200471034fbeb))
* add support for libc in windows and mac ([1fb3b5b](https://github.com/sevki/jetstream/commit/1fb3b5ba61c36372db24231f66753453a05b149a))
* add toasty types fix skip ([0fe936a](https://github.com/sevki/jetstream/commit/0fe936abdda512235343a77ff29fed7c972d3f96))
* auto-release ([964036c](https://github.com/sevki/jetstream/commit/964036cd97bfbae055f3b52653387d65524f1972))
* auto-release feature ([6505b0f](https://github.com/sevki/jetstream/commit/6505b0ff66ce16b1032efe722620d03fbe945769))
* benchmarks ([af695e2](https://github.com/sevki/jetstream/commit/af695e28faa054b8434a13ae963d2fc6abab5d3a))
* benchmarks ([94767dd](https://github.com/sevki/jetstream/commit/94767ddfbd2b43f9562621dbbd679b3ec2215da6))
* benchmarks and add iroh_benchmarks ([56dbc12](https://github.com/sevki/jetstream/commit/56dbc12c0938ed5aa1b22d8e4867beaaf7b0d587))
* bothced update ([b3b7003](https://github.com/sevki/jetstream/commit/b3b7003f565fc833804a70519aeb0741d03f34be))
* broken lock file ([6148cf7](https://github.com/sevki/jetstream/commit/6148cf776ac29ab9941574422aeda44ed01450ee))
* broken okid lockfile ([63bae5e](https://github.com/sevki/jetstream/commit/63bae5e68e8707273b13dd06a74981f635556923))
* broken release-please ([089bb22](https://github.com/sevki/jetstream/commit/089bb2277ba7025b61dbff32473d9b4b1836acd9))
* broken use statement for async_trait ([cce4df6](https://github.com/sevki/jetstream/commit/cce4df6a5a594f25fcf71eb63e51534bf8b85c3b))
* bump deps ([0dbd81b](https://github.com/sevki/jetstream/commit/0dbd81b248e3d34aee458116f7aa82cddc31d570))
* bump okid ([7ed2940](https://github.com/sevki/jetstream/commit/7ed29402ed7a2c41d2f88696cefec4a42c09ec38))
* bump zero copy ([#140](https://github.com/sevki/jetstream/issues/140)) ([4bb933f](https://github.com/sevki/jetstream/commit/4bb933f8ab6399e44e9a9ba4dc9ff92b43d69454))
* cargo lock resolver ([327d8bd](https://github.com/sevki/jetstream/commit/327d8bdfe9d11f3d4d99e245de100ffae9e5d94b))
* cargo publish workspace ([8db9367](https://github.com/sevki/jetstream/commit/8db93673748a9b5684f746ab55a34c289df70554))
* cargo toml version issues ([3481015](https://github.com/sevki/jetstream/commit/3481015e9fd2ffb3fc881168bb38ee03c72144d5))
* change service shape ([194252d](https://github.com/sevki/jetstream/commit/194252db4e3a58509845692a09a77fd9d1ccac2f))
* ci ([e7418e5](https://github.com/sevki/jetstream/commit/e7418e584a5b45965666f85bbfbf9795fc1a5b7c))
* ci release-please ([de391e5](https://github.com/sevki/jetstream/commit/de391e58d30f5f08d89f2f9251e9f734bd945bb1))
* ci workflows ([4c12f04](https://github.com/sevki/jetstream/commit/4c12f046e9ff9a5d39f4f0619f6ed68b25edcfe7))
* ci-builds, drop 1.76 ([5664e57](https://github.com/sevki/jetstream/commit/5664e571b607eb8ed3b6f9ee3488994b4cd568b9))
* ci/cd ([#157](https://github.com/sevki/jetstream/issues/157)) ([3f7ff1e](https://github.com/sevki/jetstream/commit/3f7ff1ea0b561136bc51c65f1acc0f683cfafb01))
* cleanup code and remove unused mermaid ([c8975d5](https://github.com/sevki/jetstream/commit/c8975d56c388470fb4305c569ee30b84fde30f81))
* collections can only be u16::MAX ([6ada71a](https://github.com/sevki/jetstream/commit/6ada71a7222d16b0e3035837c76b8e3bcb51f627))
* criterion benchmark test, framed implementation ([25b1611](https://github.com/sevki/jetstream/commit/25b1611be97ffde52a1cb403ea4a55e3aa5f8e0b))
* Delete CHANGELOG.md ([abcc79d](https://github.com/sevki/jetstream/commit/abcc79dc0511230f4725ea5da20c624352635b65))
* dependency order ([9a0e1f9](https://github.com/sevki/jetstream/commit/9a0e1f979c85a45780bf71f669da4cc91ecd3f5b))
* dependency order and cycle ([6a6c997](https://github.com/sevki/jetstream/commit/6a6c99778f90f8481d5792d560a6e97e0c5926bc))
* dependency sudoku ([7df5f9b](https://github.com/sevki/jetstream/commit/7df5f9bdce4a3a830609fe5f50bfc3cdd278b5d2))
* disable running echo on windows ([828d6be](https://github.com/sevki/jetstream/commit/828d6be26328a938417b976cb8c8f63ddc1a9bea))
* disable running echo on windows ([7f44956](https://github.com/sevki/jetstream/commit/7f44956f965d5c906812e91b138e5c21597e37df))
* docs ([995da2f](https://github.com/sevki/jetstream/commit/995da2f79e1fc42eeb8a87811d070094b6e9e4b1))
* elide lifetimes in ufs ([b778ff9](https://github.com/sevki/jetstream/commit/b778ff9481bf9c9d1cf35fc691903b80483e8cdb))
* extern async_trait and trait_variant and lazy_static ([71e93e2](https://github.com/sevki/jetstream/commit/71e93e2db32236e01bfc2cc923a6db10a788a730))
* extern_more ([1bd58d8](https://github.com/sevki/jetstream/commit/1bd58d8150031c1394f94e7074afd2c1c60928de))
* failing docs ([232ddee](https://github.com/sevki/jetstream/commit/232ddee45d616dc08ea7e6dcd072db2c7f2cbba6))
* failing serde tests ([81db0de](https://github.com/sevki/jetstream/commit/81db0ded08429489cf5d83d5540804b00fdb9802))
* filesystem under feature flag, rm newline ([de4cf79](https://github.com/sevki/jetstream/commit/de4cf791a89b794dbfcb48325b1a90f26e421616))
* formatting ([049e584](https://github.com/sevki/jetstream/commit/049e58478f81be41983bdeef4a661f12eb21215a))
* formatting ([04aace4](https://github.com/sevki/jetstream/commit/04aace42d2d1338e614081954e32448ac3e5ec30))
* fuzz target ([a494e15](https://github.com/sevki/jetstream/commit/a494e15fca5b0ee1e66399d2b50cf0e1158b070c))
* hide documentation for iroh ([47b104d](https://github.com/sevki/jetstream/commit/47b104d73ece751924c1e41a77a92a8317c968f9))
* ignore e2e tests ([e066dde](https://github.com/sevki/jetstream/commit/e066dde3f735c2524118ee0e8128555775d0eeb7))
* iroh example needs -F iroh ([5246fb2](https://github.com/sevki/jetstream/commit/5246fb234453964ebf0e0f12ad0437f23780032d))
* keep reciver types as is in generated code ([7d95671](https://github.com/sevki/jetstream/commit/7d956718c4e4c1564b1aeeb2d6af635eb44ae220))
* lint errors ([4f50d0b](https://github.com/sevki/jetstream/commit/4f50d0b73ddcfeaee2e86a2d27d061d87a0c1134))
* **macros:** protocol macro fix to spread fn args ([b261a28](https://github.com/sevki/jetstream/commit/b261a286e033e2f9ba1462dc8a5dd06adf4e5ca3))
* make data io::Read ([12a864e](https://github.com/sevki/jetstream/commit/12a864e93e402c0145ff1f206589164bc920fbdd))
* make data io::Read ([910c75a](https://github.com/sevki/jetstream/commit/910c75ad120d6b699691dc194377138837b2a8f5))
* make data io::Read ([77b3680](https://github.com/sevki/jetstream/commit/77b3680ff8ac312b14303f864d92bb763659b64f))
* make error types more isomorphic ([aee2fa2](https://github.com/sevki/jetstream/commit/aee2fa21a7c6879f5d77765d3ff96b22b98e0e86))
* make qid eq,hash ([522de0d](https://github.com/sevki/jetstream/commit/522de0d41e5cbda8d29ea6a20f1cb2cba7b5bb82))
* make the proto mod same vis as trait ([a43c0a2](https://github.com/sevki/jetstream/commit/a43c0a2e3fbcb2991f6f3396eba9e30c96a47663))
* make websocket pub ([6096ff9](https://github.com/sevki/jetstream/commit/6096ff9815b3b15dac1cfdd124821dc9d40218b0))
* make websocket pub again ([21acf4b](https://github.com/sevki/jetstream/commit/21acf4bd6f9269ff9c0ce3b4c3654bbfa2ca6d98))
* mdbook changelog ([5f03b6b](https://github.com/sevki/jetstream/commit/5f03b6b7e2fe7b2fb56f082f8ff56105066b0998))
* mdbook-changelog version issue ([7043b8e](https://github.com/sevki/jetstream/commit/7043b8e1b3ef85b1da383e6e2488bc32e4c9f839))
* more tests ([9a5071f](https://github.com/sevki/jetstream/commit/9a5071fb22c4a63e0af4fb1af9c7133f00cbc26e))
* move term-transcript to dev, gate s2n windows ([5fa7f4c](https://github.com/sevki/jetstream/commit/5fa7f4ce55ef111c0a155416d556189fbc266b87))
* only build docs in release ([02b02bc](https://github.com/sevki/jetstream/commit/02b02bc6348a3ec537683d9b18047c02e8b7c502))
* option&lt;T&gt; support ([2e224ca](https://github.com/sevki/jetstream/commit/2e224cad47d775fb2c967c4d37d33b8aabeb7470))
* pin toasty ([03b6b18](https://github.com/sevki/jetstream/commit/03b6b18c6cce3cf9cdb274040bf29da21e1e5dd2))
* publish script ([e8685cc](https://github.com/sevki/jetstream/commit/e8685cc195682d2467de6d042ff3052ddd9175c5))
* readme ([dc77722](https://github.com/sevki/jetstream/commit/dc777223fe95ea79866320b7831e4e3c89631820))
* recursive self call ([c51d455](https://github.com/sevki/jetstream/commit/c51d455ececd5ac461561b8bd44709a92a45b2b7))
* reenable sccache ([757bb7e](https://github.com/sevki/jetstream/commit/757bb7e248996a587ee13c2a8c21246d0e198a04))
* reexport tokio_util from rpc ([cf28c40](https://github.com/sevki/jetstream/commit/cf28c40a3783379ac89295fef4a55bd4212d4891))
* release again ([3a6e65e](https://github.com/sevki/jetstream/commit/3a6e65e2d8181bcc93a41a577e2ca04c9ca70bf2))
* release please ([53a698b](https://github.com/sevki/jetstream/commit/53a698b53ad31d12cc068ba8e921779b710a448e))
* release please ([92eeee5](https://github.com/sevki/jetstream/commit/92eeee530c2be541a31a46d86dcd67fd68b0c8a5))
* release please ([3a2f4df](https://github.com/sevki/jetstream/commit/3a2f4df4bbec17361295a2839ba10e521656f94f))
* release workflow ([4abeb24](https://github.com/sevki/jetstream/commit/4abeb24e99aec6ae9ddcc083136cc765b84ba094))
* release-please-config.json ([885527f](https://github.com/sevki/jetstream/commit/885527f075deffba2f139739032460e35d93bc08))
* remove distributed ([3aba1f6](https://github.com/sevki/jetstream/commit/3aba1f6543493e107b079d35fa441d46611e637e))
* remove expects ([8a58cf5](https://github.com/sevki/jetstream/commit/8a58cf565453d755100b5298c1c89ed478dff11e))
* remove jj ([02fefd9](https://github.com/sevki/jetstream/commit/02fefd9e82f0f260d54f703daa0eb4667cdbce5d))
* remove prost ([5a5ff9b](https://github.com/sevki/jetstream/commit/5a5ff9b7904f1023f5bbed6c5a6236ef01644c17))
* remove redundant io imports and use fully qualified paths ([7de4b9e](https://github.com/sevki/jetstream/commit/7de4b9ea0cece01326cde4af027eae8a3120f1f9))
* remove tracing from sink and stream ([97ac0a5](https://github.com/sevki/jetstream/commit/97ac0a59b3c0cd8aefb2b0af2804b81316ea6acf))
* remove unsafe code ([a286d93](https://github.com/sevki/jetstream/commit/a286d93fddc7bf8329defe77b5d01c0b6dbabd17))
* revert serde_bytes to bytes with serde ([2e02460](https://github.com/sevki/jetstream/commit/2e02460c71890a59af315f29897d4edbe79f9c54))
* rollback toasty ([ae0790b](https://github.com/sevki/jetstream/commit/ae0790b473fa81e1c67ec25003bec0f0d6082450))
* rust.yml workflow ([9dc7bc0](https://github.com/sevki/jetstream/commit/9dc7bc09ad8a341e8bcf19f945b8efa478b0d67d))
* rustdoc to mdbook ([2098ad1](https://github.com/sevki/jetstream/commit/2098ad11072e31ec329b42ae7df6dd380d993ff7))
* service macro works correctly with client calls ([eb1fd0f](https://github.com/sevki/jetstream/commit/eb1fd0f01bc899b7f0650dbfca29e2d77ec76720))
* simpler workflows for github actions ([17185dd](https://github.com/sevki/jetstream/commit/17185ddd3337fff27131f96a26eabcb0d1961155))
* snapshots for macros ([fab686d](https://github.com/sevki/jetstream/commit/fab686df8c79710b5e9319519d8aa3d17b203847))
* some tests ([c65ded1](https://github.com/sevki/jetstream/commit/c65ded107a254d283e165563b0b97fa735741399))
* target os cfg attrs ([16e5c84](https://github.com/sevki/jetstream/commit/16e5c849b2c1b7b962590dfab779f4429cf06445))
* trait_variant use ([#155](https://github.com/sevki/jetstream/issues/155)) ([3cd5665](https://github.com/sevki/jetstream/commit/3cd5665454bb42c3e38606c4a3dd9355ef63fc9b))
* typo ([901ffbd](https://github.com/sevki/jetstream/commit/901ffbdda65336060410f348fe57c0ddc01a9fc4))
* unused git ([3fe908e](https://github.com/sevki/jetstream/commit/3fe908e405bc870a8089b18e88dae245f41eeb68))
* update 9p to use trait_variant ([96db410](https://github.com/sevki/jetstream/commit/96db410d8e5e04c4beb9100be2b58c271cdd7b8c))
* Update CHANGELOG.md ([6ab562f](https://github.com/sevki/jetstream/commit/6ab562f5210fa30a0a33a393d4fa18f3eb1154db))
* Update client_tests.rs ([4c50132](https://github.com/sevki/jetstream/commit/4c50132ba0d6afa46f849db7d0fa356e64947653))
* update doc links ([80a5e1e](https://github.com/sevki/jetstream/commit/80a5e1efc9b2a275b8d49be7eedc832602914c0c))
* update docs ([74e2867](https://github.com/sevki/jetstream/commit/74e2867864e46b441ed01193220c22842fcf67a8))
* Update lib.rs for extern crates ([ffb6777](https://github.com/sevki/jetstream/commit/ffb677744364c3d44274be2d5eb20c30ecabb6b5))
* update release flow ([#144](https://github.com/sevki/jetstream/issues/144)) ([36dd4af](https://github.com/sevki/jetstream/commit/36dd4af7686d405eeaa42d53e7eaac934d901489))
* Update release-please-config.json ([e592cf3](https://github.com/sevki/jetstream/commit/e592cf3ed85bf657b26e792e149848b5623bbd5b))
* Update release.yml ([b058b38](https://github.com/sevki/jetstream/commit/b058b38367db97630b91af82e6bb11e26728708e))
* Update release.yml ([#122](https://github.com/sevki/jetstream/issues/122)) ([566fe1f](https://github.com/sevki/jetstream/commit/566fe1f64c6d6f25d157a227b261c69c9b3e83b5))
* update to v2 upload sarif ([e38bacb](https://github.com/sevki/jetstream/commit/e38bacb216db9b6560b3fe0e284b54425b4e251e))
* use cargo publish workspace ([e28a0f7](https://github.com/sevki/jetstream/commit/e28a0f75bfa9fca9a990c6f2b5527be5e25a3d16))
* version ([ca861a7](https://github.com/sevki/jetstream/commit/ca861a7ac9797925d208c3b08e8d195607d4d388))
* version ([822bf0e](https://github.com/sevki/jetstream/commit/822bf0ed14786912a6ca9a7329a577f6d2367945))
* versions in workspace ([8b90b5a](https://github.com/sevki/jetstream/commit/8b90b5af219a36fa4934ace0e6411baedf816843))
* warnings ([62d8013](https://github.com/sevki/jetstream/commit/62d8013e3b9fa2c0120cd9269894ed640c03fb83))
* wasm-pack build error. rm rustdoc2mdbook ([68076cc](https://github.com/sevki/jetstream/commit/68076ccbcf88d18d1dadb30355d8358283864b54))
* wasm32 feature gates ([755f9c1](https://github.com/sevki/jetstream/commit/755f9c1c6628fc46b1d4d3df03441106de3abdfd))
* **wireformat:** from_bytes doesn't require a mutable buf ([437c35c](https://github.com/sevki/jetstream/commit/437c35c2229cd5980f752573fda43c1d6dfae725))


### Code Refactoring

* merge all the creates ([faa0a1a](https://github.com/sevki/jetstream/commit/faa0a1a1194bac41d8e05efd0108e0c1821fa543))
* move modules to sensible parents ([4eba5fb](https://github.com/sevki/jetstream/commit/4eba5fb0105626fd56c42f2062c7cfbe7293279b))
* protocol -&gt; coding ([5f86bc7](https://github.com/sevki/jetstream/commit/5f86bc78a85728091f8411ab00f5ad3e4a960df2))

## [10.1.0](https://github.com/sevki/jetstream/compare/v10.0.0...v10.1.0) (2026-01-20)


### Features

* better error handling ([faae10a](https://github.com/sevki/jetstream/commit/faae10a704821d723be55e0e6d92e88def17591c))
* update mdbook ([f213c71](https://github.com/sevki/jetstream/commit/f213c712bcc5395b048f83b2c352c163c11518e0))


### Bug Fixes

* benchmarks ([af695e2](https://github.com/sevki/jetstream/commit/af695e28faa054b8434a13ae963d2fc6abab5d3a))
* cargo publish workspace ([8db9367](https://github.com/sevki/jetstream/commit/8db93673748a9b5684f746ab55a34c289df70554))
* cleanup code and remove unused mermaid ([c8975d5](https://github.com/sevki/jetstream/commit/c8975d56c388470fb4305c569ee30b84fde30f81))
* mdbook-changelog version issue ([7043b8e](https://github.com/sevki/jetstream/commit/7043b8e1b3ef85b1da383e6e2488bc32e4c9f839))
* move term-transcript to dev, gate s2n windows ([5fa7f4c](https://github.com/sevki/jetstream/commit/5fa7f4ce55ef111c0a155416d556189fbc266b87))
* publish script ([e8685cc](https://github.com/sevki/jetstream/commit/e8685cc195682d2467de6d042ff3052ddd9175c5))

## [10.0.0](https://github.com/sevki/jetstream/compare/v9.5.0...v10.0.0) (2025-11-19)


### ⚠ BREAKING CHANGES

* make error types more isomorphic

### Bug Fixes

* make error types more isomorphic ([aee2fa2](https://github.com/sevki/jetstream/commit/aee2fa21a7c6879f5d77765d3ff96b22b98e0e86))

## [9.5.0](https://github.com/sevki/jetstream/compare/v9.4.2...v9.5.0) (2025-10-30)


### Features

* add cleanup code serverside. add cloudflare docs to mdbook ([91da99c](https://github.com/sevki/jetstream/commit/91da99cae4a47dbd80672574e75098909bbcb79b))
* introduce jetstream cloudflare module. ([6e8cc01](https://github.com/sevki/jetstream/commit/6e8cc015024d3066f9f3208e20bd4725c03b0ac2))


### Bug Fixes

* add comment about cf exec model ([c979856](https://github.com/sevki/jetstream/commit/c979856a7406e0f612fc26adbfa200471034fbeb))
* remove tracing from sink and stream ([97ac0a5](https://github.com/sevki/jetstream/commit/97ac0a59b3c0cd8aefb2b0af2804b81316ea6acf))
* remove unsafe code ([a286d93](https://github.com/sevki/jetstream/commit/a286d93fddc7bf8329defe77b5d01c0b6dbabd17))

## [9.4.2](https://github.com/sevki/jetstream/compare/v9.4.1...v9.4.2) (2025-10-28)


### Bug Fixes

* dependency sudoku ([7df5f9b](https://github.com/sevki/jetstream/commit/7df5f9bdce4a3a830609fe5f50bfc3cdd278b5d2))

## [9.4.1](https://github.com/sevki/jetstream/compare/v9.4.0...v9.4.1) (2025-10-17)


### Bug Fixes

* remove redundant io imports and use fully qualified paths ([7de4b9e](https://github.com/sevki/jetstream/commit/7de4b9ea0cece01326cde4af027eae8a3120f1f9))
* target os cfg attrs ([16e5c84](https://github.com/sevki/jetstream/commit/16e5c849b2c1b7b962590dfab779f4429cf06445))

## [9.4.0](https://github.com/sevki/jetstream/compare/v9.3.0...v9.4.0) (2025-10-15)


### Features

* add context ([4771c84](https://github.com/sevki/jetstream/commit/4771c84d8a7f3a35ff1b8c25ebe0b0871dd7592d))


### Bug Fixes

* cargo toml version issues ([3481015](https://github.com/sevki/jetstream/commit/3481015e9fd2ffb3fc881168bb38ee03c72144d5))
* collections can only be u16::MAX ([6ada71a](https://github.com/sevki/jetstream/commit/6ada71a7222d16b0e3035837c76b8e3bcb51f627))
* recursive self call ([c51d455](https://github.com/sevki/jetstream/commit/c51d455ececd5ac461561b8bd44709a92a45b2b7))
* wasm32 feature gates ([755f9c1](https://github.com/sevki/jetstream/commit/755f9c1c6628fc46b1d4d3df03441106de3abdfd))

## [9.3.0](https://github.com/sevki/jetstream/compare/v9.2.0...v9.3.0) (2025-10-14)


### Features

* add tracing feature ([5cb2907](https://github.com/sevki/jetstream/commit/5cb290799dd251c6e04ab4c2fe52089e3f32336c))

## [9.2.0](https://github.com/sevki/jetstream/compare/v9.1.2...v9.2.0) (2025-10-05)


### Features

* dt features, remove rustdoc_to_md_book ([0398159](https://github.com/sevki/jetstream/commit/039815903e127055ec494a3dc3e1780b7afb4fe2))


### Bug Fixes

* iroh example needs -F iroh ([5246fb2](https://github.com/sevki/jetstream/commit/5246fb234453964ebf0e0f12ad0437f23780032d))
* wasm-pack build error. rm rustdoc2mdbook ([68076cc](https://github.com/sevki/jetstream/commit/68076ccbcf88d18d1dadb30355d8358283864b54))

## [9.1.2](https://github.com/sevki/jetstream/compare/v9.1.1...v9.1.2) (2025-10-05)


### Bug Fixes

* failing docs ([232ddee](https://github.com/sevki/jetstream/commit/232ddee45d616dc08ea7e6dcd072db2c7f2cbba6))

## [9.1.1](https://github.com/sevki/jetstream/compare/v9.1.0...v9.1.1) (2025-10-05)


### Bug Fixes

* benchmarks and add iroh_benchmarks ([56dbc12](https://github.com/sevki/jetstream/commit/56dbc12c0938ed5aa1b22d8e4867beaaf7b0d587))
* hide documentation for iroh ([47b104d](https://github.com/sevki/jetstream/commit/47b104d73ece751924c1e41a77a92a8317c968f9))

## [9.1.0](https://github.com/sevki/jetstream/compare/v9.0.0...v9.1.0) (2025-10-02)


### Features

* introduce iroh examples ([86e001f](https://github.com/sevki/jetstream/commit/86e001f9195a28429e34670597347d3148ea22d9))


### Bug Fixes

* add support for libc in windows and mac ([1fb3b5b](https://github.com/sevki/jetstream/commit/1fb3b5ba61c36372db24231f66753453a05b149a))
* disable running echo on windows ([828d6be](https://github.com/sevki/jetstream/commit/828d6be26328a938417b976cb8c8f63ddc1a9bea))
* disable running echo on windows ([7f44956](https://github.com/sevki/jetstream/commit/7f44956f965d5c906812e91b138e5c21597e37df))

## [9.0.0](https://github.com/sevki/jetstream/compare/v8.3.1...v9.0.0) (2025-09-29)


### ⚠ BREAKING CHANGES

* move quic and webocket to their own crates.

### Features

* move quic and webocket to their own crates. ([7d0ba9f](https://github.com/sevki/jetstream/commit/7d0ba9fae68f1e4995d9ea6d1adcaf95b8dc983a))


### Bug Fixes

* simpler workflows for github actions ([17185dd](https://github.com/sevki/jetstream/commit/17185ddd3337fff27131f96a26eabcb0d1961155))

## [8.3.1](https://github.com/sevki/jetstream/compare/v8.3.0...v8.3.1) (2025-06-06)


### Bug Fixes

* update docs ([74e2867](https://github.com/sevki/jetstream/commit/74e2867864e46b441ed01193220c22842fcf67a8))

## [8.3.0](https://github.com/sevki/jetstream/compare/v8.2.1...v8.3.0) (2025-05-31)


### Features

* jetstream_libc ([aba123b](https://github.com/sevki/jetstream/commit/aba123bb7a027a97cad4d8a9d7731d18bb4059ca))

## [8.2.1](https://github.com/sevki/jetstream/compare/v8.2.0...v8.2.1) (2025-05-29)


### Bug Fixes

* rustdoc to mdbook ([2098ad1](https://github.com/sevki/jetstream/commit/2098ad11072e31ec329b42ae7df6dd380d993ff7))

## [8.2.0](https://github.com/sevki/jetstream/compare/v8.1.5...v8.2.0) (2025-05-29)


### Features

* jetstream distributed features ([52b5e89](https://github.com/sevki/jetstream/commit/52b5e89e64b1e5559e600bed5fd91499cd051934))


### Bug Fixes

* Update lib.rs for extern crates ([ffb6777](https://github.com/sevki/jetstream/commit/ffb677744364c3d44274be2d5eb20c30ecabb6b5))

## [8.1.5](https://github.com/sevki/jetstream/compare/v8.1.4...v8.1.5) (2025-04-05)


### Bug Fixes

* rollback toasty ([ae0790b](https://github.com/sevki/jetstream/commit/ae0790b473fa81e1c67ec25003bec0f0d6082450))
* version ([ca861a7](https://github.com/sevki/jetstream/commit/ca861a7ac9797925d208c3b08e8d195607d4d388))

## [8.1.4](https://github.com/sevki/jetstream/compare/v8.1.3...v8.1.4) (2025-04-04)


### Bug Fixes

* add toasty types fix skip ([0fe936a](https://github.com/sevki/jetstream/commit/0fe936abdda512235343a77ff29fed7c972d3f96))
* ci-builds, drop 1.76 ([5664e57](https://github.com/sevki/jetstream/commit/5664e571b607eb8ed3b6f9ee3488994b4cd568b9))
* pin toasty ([03b6b18](https://github.com/sevki/jetstream/commit/03b6b18c6cce3cf9cdb274040bf29da21e1e5dd2))

## [8.1.3](https://github.com/sevki/jetstream/compare/v8.1.2...v8.1.3) (2025-04-04)


### Bug Fixes

* extern async_trait and trait_variant and lazy_static ([71e93e2](https://github.com/sevki/jetstream/commit/71e93e2db32236e01bfc2cc923a6db10a788a730))
* extern_more ([1bd58d8](https://github.com/sevki/jetstream/commit/1bd58d8150031c1394f94e7074afd2c1c60928de))

## [8.1.2](https://github.com/sevki/jetstream/compare/v8.1.1...v8.1.2) (2025-02-15)


### Bug Fixes

* make websocket pub again ([21acf4b](https://github.com/sevki/jetstream/commit/21acf4bd6f9269ff9c0ce3b4c3654bbfa2ca6d98))

## [8.1.1](https://github.com/sevki/jetstream/compare/v8.1.0...v8.1.1) (2025-02-15)


### Bug Fixes

* make websocket pub ([6096ff9](https://github.com/sevki/jetstream/commit/6096ff9815b3b15dac1cfdd124821dc9d40218b0))

## [8.1.0](https://github.com/sevki/jetstream/compare/v8.0.10...v8.1.0) (2025-02-15)


### Features

* add websocket transport ([3f4054c](https://github.com/sevki/jetstream/commit/3f4054c9ebe2253c11a2096c4983c7d75073da9a))
* channels can now be split ([ef646a5](https://github.com/sevki/jetstream/commit/ef646a590a5af13adc2c97c4871f19bca2a1d6b6))

## [8.0.10](https://github.com/sevki/jetstream/compare/v8.0.9...v8.0.10) (2025-02-14)


### Bug Fixes

* broken lock file ([6148cf7](https://github.com/sevki/jetstream/commit/6148cf776ac29ab9941574422aeda44ed01450ee))

## [8.0.9](https://github.com/sevki/jetstream/compare/v8.0.8...v8.0.9) (2025-02-14)


### Bug Fixes

* remove prost ([5a5ff9b](https://github.com/sevki/jetstream/commit/5a5ff9b7904f1023f5bbed6c5a6236ef01644c17))

## [8.0.8](https://github.com/sevki/jetstream/compare/v8.0.7...v8.0.8) (2025-02-01)


### Bug Fixes

* remove expects ([8a58cf5](https://github.com/sevki/jetstream/commit/8a58cf565453d755100b5298c1c89ed478dff11e))

## [8.0.7](https://github.com/sevki/jetstream/compare/v8.0.6...v8.0.7) (2025-01-30)


### Bug Fixes

* release please ([53a698b](https://github.com/sevki/jetstream/commit/53a698b53ad31d12cc068ba8e921779b710a448e))

## [8.0.6](https://github.com/sevki/jetstream/compare/v8.0.5...v8.0.6) (2025-01-30)


### Bug Fixes

* release please ([92eeee5](https://github.com/sevki/jetstream/commit/92eeee530c2be541a31a46d86dcd67fd68b0c8a5))
* release please ([3a2f4df](https://github.com/sevki/jetstream/commit/3a2f4df4bbec17361295a2839ba10e521656f94f))

## [8.0.5](https://github.com/sevki/jetstream/compare/v8.0.4...v8.0.5) (2025-01-30)


### Bug Fixes

* use cargo publish workspace ([e28a0f7](https://github.com/sevki/jetstream/commit/e28a0f75bfa9fca9a990c6f2b5527be5e25a3d16))

## [8.0.4](https://github.com/sevki/jetstream/compare/v8.0.3...v8.0.4) (2025-01-30)


### Bug Fixes

* dependency order and cycle ([6a6c997](https://github.com/sevki/jetstream/commit/6a6c99778f90f8481d5792d560a6e97e0c5926bc))
* formatting ([049e584](https://github.com/sevki/jetstream/commit/049e58478f81be41983bdeef4a661f12eb21215a))
* reexport tokio_util from rpc ([cf28c40](https://github.com/sevki/jetstream/commit/cf28c40a3783379ac89295fef4a55bd4212d4891))
* rust.yml workflow ([9dc7bc0](https://github.com/sevki/jetstream/commit/9dc7bc09ad8a341e8bcf19f945b8efa478b0d67d))

## [8.0.3](https://github.com/sevki/jetstream/compare/v8.0.2...v8.0.3) (2025-01-30)


### Bug Fixes

* cargo lock resolver ([327d8bd](https://github.com/sevki/jetstream/commit/327d8bdfe9d11f3d4d99e245de100ffae9e5d94b))
* dependency order ([9a0e1f9](https://github.com/sevki/jetstream/commit/9a0e1f979c85a45780bf71f669da4cc91ecd3f5b))

## [8.0.2](https://github.com/sevki/jetstream/compare/v8.0.1...v8.0.2) (2025-01-30)


### Bug Fixes

* criterion benchmark test, framed implementation ([25b1611](https://github.com/sevki/jetstream/commit/25b1611be97ffde52a1cb403ea4a55e3aa5f8e0b))
* failing serde tests ([81db0de](https://github.com/sevki/jetstream/commit/81db0ded08429489cf5d83d5540804b00fdb9802))
* reenable sccache ([757bb7e](https://github.com/sevki/jetstream/commit/757bb7e248996a587ee13c2a8c21246d0e198a04))
* service macro works correctly with client calls ([eb1fd0f](https://github.com/sevki/jetstream/commit/eb1fd0f01bc899b7f0650dbfca29e2d77ec76720))
* snapshots for macros ([fab686d](https://github.com/sevki/jetstream/commit/fab686df8c79710b5e9319519d8aa3d17b203847))

## [8.0.1](https://github.com/sevki/jetstream/compare/v8.0.0...v8.0.1) (2025-01-28)


### Bug Fixes

* remove distributed ([3aba1f6](https://github.com/sevki/jetstream/commit/3aba1f6543493e107b079d35fa441d46611e637e))

## [8.0.0](https://github.com/sevki/jetstream/compare/v7.4.0...v8.0.0) (2025-01-27)


### ⚠ BREAKING CHANGES

* use more futures

### Features

* use more futures ([467b6f5](https://github.com/sevki/jetstream/commit/467b6f56d0808e89f078c656f1e7c447738244ed))


### Bug Fixes

* formatting ([04aace4](https://github.com/sevki/jetstream/commit/04aace42d2d1338e614081954e32448ac3e5ec30))

## [7.4.0](https://github.com/sevki/jetstream/compare/v7.3.0...v7.4.0) (2025-01-18)


### Features

* jetstream_rpc supports wasm ([e97e6ca](https://github.com/sevki/jetstream/commit/e97e6ca6a94067551fb7a98c4e34fa2a774ebdfd))

## [7.3.0](https://github.com/sevki/jetstream/compare/v7.2.1...v7.3.0) (2025-01-17)


### Features

* wasm-support ([#260](https://github.com/sevki/jetstream/issues/260)) ([5cbff0d](https://github.com/sevki/jetstream/commit/5cbff0daf95c780cd5962252aaa975d6248b524b))

## [7.2.1](https://github.com/sevki/jetstream/compare/v7.2.0...v7.2.1) (2024-12-10)


### Bug Fixes

* keep reciver types as is in generated code ([7d95671](https://github.com/sevki/jetstream/commit/7d956718c4e4c1564b1aeeb2d6af635eb44ae220))

## [7.2.0](https://github.com/sevki/jetstream/compare/v7.1.2...v7.2.0) (2024-12-09)


### Features

* Ip primitives ([1c263b5](https://github.com/sevki/jetstream/commit/1c263b59494e2699ec2347727f6cbf77bdac3b67))

## [7.1.2](https://github.com/sevki/jetstream/compare/v7.1.1...v7.1.2) (2024-12-06)


### Bug Fixes

* elide lifetimes in ufs ([b778ff9](https://github.com/sevki/jetstream/commit/b778ff9481bf9c9d1cf35fc691903b80483e8cdb))

## [7.1.1](https://github.com/sevki/jetstream/compare/v7.1.0...v7.1.1) (2024-11-25)


### Bug Fixes

* broken use statement for async_trait ([cce4df6](https://github.com/sevki/jetstream/commit/cce4df6a5a594f25fcf71eb63e51534bf8b85c3b))

## [7.1.0](https://github.com/sevki/jetstream/compare/v7.0.4...v7.1.0) (2024-11-25)


### Features

* added async_trait support service macro ([9a86185](https://github.com/sevki/jetstream/commit/9a86185b07eb02cd05933920f9ad02792992c71c))

## [7.0.4](https://github.com/sevki/jetstream/compare/v7.0.3...v7.0.4) (2024-11-23)


### Bug Fixes

* fuzz target ([a494e15](https://github.com/sevki/jetstream/commit/a494e15fca5b0ee1e66399d2b50cf0e1158b070c))

## [7.0.3](https://github.com/sevki/jetstream/compare/v7.0.2...v7.0.3) (2024-11-21)


### Bug Fixes

* Delete CHANGELOG.md ([abcc79d](https://github.com/sevki/jetstream/commit/abcc79dc0511230f4725ea5da20c624352635b65))
* mdbook changelog ([5f03b6b](https://github.com/sevki/jetstream/commit/5f03b6b7e2fe7b2fb56f082f8ff56105066b0998))

## [7.0.1](https://github.com/sevki/jetstream/compare/v7.0.0...v7.0.1) (2024-11-21)


### Bug Fixes

* ci ([e7418e5](https://github.com/sevki/jetstream/commit/e7418e584a5b45965666f85bbfbf9795fc1a5b7c))
* docs ([995da2f](https://github.com/sevki/jetstream/commit/995da2f79e1fc42eeb8a87811d070094b6e9e4b1))

## [7.0.0](https://github.com/sevki/jetstream/compare/v6.6.2...v7.0.0) (2024-11-21)


### ⚠ BREAKING CHANGES

* fix proken publish

### Features

* fix proken publish ([a7272c0](https://github.com/sevki/jetstream/commit/a7272c0f3dc25aa10e89c8b7e9ad070d919a369d))

## [6.6.2](https://github.com/sevki/jetstream/compare/v6.6.1...v6.6.2) (2024-11-21)


### Bug Fixes

* bump okid ([7ed2940](https://github.com/sevki/jetstream/commit/7ed29402ed7a2c41d2f88696cefec4a42c09ec38))

## [6.6.1](https://github.com/sevki/jetstream/compare/v6.6.0...v6.6.1) (2024-11-20)


### Bug Fixes

* broken okid lockfile ([63bae5e](https://github.com/sevki/jetstream/commit/63bae5e68e8707273b13dd06a74981f635556923))

## [6.6.0](https://github.com/sevki/jetstream/compare/v6.5.0...v6.6.0) (2024-11-20)


### Features

* distributes jetsream ([d1477d5](https://github.com/sevki/jetstream/commit/d1477d56e574ca4c76b967a27c84ce7ccc4d9396))
* **jetstream_distributed:** placement ([8a93788](https://github.com/sevki/jetstream/commit/8a93788e51a98807466aa868b13685ba81aa1d3e))


### Bug Fixes

* change service shape ([194252d](https://github.com/sevki/jetstream/commit/194252db4e3a58509845692a09a77fd9d1ccac2f))
* typo ([901ffbd](https://github.com/sevki/jetstream/commit/901ffbdda65336060410f348fe57c0ddc01a9fc4))


## [6.5.0](https://github.com/sevki/jetstream/compare/v6.4.2...v6.5.0) (2024-11-20)


### Features

* jetstream cluster ([80b5727](https://github.com/sevki/jetstream/commit/80b5727722b1b44d522eb793c18e5bf117660d8b))
* rustdoc_to_mdbook ([32deadf](https://github.com/sevki/jetstream/commit/32deadf98614a9365e9991c3cd76620280c85b5d))


### Bug Fixes

* only build docs in release ([02b02bc](https://github.com/sevki/jetstream/commit/02b02bc6348a3ec537683d9b18047c02e8b7c502))
* update doc links ([80a5e1e](https://github.com/sevki/jetstream/commit/80a5e1efc9b2a275b8d49be7eedc832602914c0c))

## [6.4.2](https://github.com/sevki/jetstream/compare/v6.4.1...v6.4.2) (2024-11-18)


### Bug Fixes

* bump deps ([0dbd81b](https://github.com/sevki/jetstream/commit/0dbd81b248e3d34aee458116f7aa82cddc31d570))

## [6.4.1](https://github.com/sevki/jetstream/compare/v6.4.0...v6.4.1) (2024-11-11)


### Bug Fixes

* make the proto mod same vis as trait ([a43c0a2](https://github.com/sevki/jetstream/commit/a43c0a2e3fbcb2991f6f3396eba9e30c96a47663))

## [6.4.0](https://github.com/sevki/jetstream/compare/v6.3.4...v6.4.0) (2024-11-10)


### Features

* Add support for bool in WireFormat ([640c6ca](https://github.com/sevki/jetstream/commit/640c6ca3e22d0ed1dc7ec9c58af998e96efab235))
* Add tests for jetstream modules ([18462d9](https://github.com/sevki/jetstream/commit/18462d96f8dd71b528d49441dc257d2038f3e39f))


### Bug Fixes

* benchmarks ([94767dd](https://github.com/sevki/jetstream/commit/94767ddfbd2b43f9562621dbbd679b3ec2215da6))
* more tests ([9a5071f](https://github.com/sevki/jetstream/commit/9a5071fb22c4a63e0af4fb1af9c7133f00cbc26e))
* some tests ([c65ded1](https://github.com/sevki/jetstream/commit/c65ded107a254d283e165563b0b97fa735741399))

## [6.3.4](https://github.com/sevki/jetstream/compare/v6.3.3...v6.3.4) (2024-11-10)


### Bug Fixes

* make qid eq,hash ([522de0d](https://github.com/sevki/jetstream/commit/522de0d41e5cbda8d29ea6a20f1cb2cba7b5bb82))

## [6.3.3](https://github.com/sevki/jetstream/compare/v6.3.2...v6.3.3) (2024-11-10)


### Bug Fixes

* update 9p to use trait_variant ([96db410](https://github.com/sevki/jetstream/commit/96db410d8e5e04c4beb9100be2b58c271cdd7b8c))

## [6.3.2](https://github.com/sevki/jetstream/compare/v6.3.1...v6.3.2) (2024-11-10)


### Bug Fixes

* option&lt;T&gt; support ([2e224ca](https://github.com/sevki/jetstream/commit/2e224cad47d775fb2c967c4d37d33b8aabeb7470))

## [6.3.1](https://github.com/sevki/jetstream/compare/v6.3.0...v6.3.1) (2024-11-09)


### Bug Fixes

* ci workflows ([4c12f04](https://github.com/sevki/jetstream/commit/4c12f046e9ff9a5d39f4f0619f6ed68b25edcfe7))

## [6.3.0](https://github.com/sevki/jetstream/compare/v6.2.0...v6.3.0) (2024-11-09)


### Features

* add i16,i32,i64,i128 types ([ba741bd](https://github.com/sevki/jetstream/commit/ba741bd0d2aed8d33d2b5589ca2a3da83b1abce8))
* enum support ([c4552d8](https://github.com/sevki/jetstream/commit/c4552d8bd056debdbb7845145f964defae6f90d2))


### Bug Fixes

* remove jj ([02fefd9](https://github.com/sevki/jetstream/commit/02fefd9e82f0f260d54f703daa0eb4667cdbce5d))

## [6.2.0](https://github.com/sevki/jetstream/compare/v6.1.0...v6.2.0) (2024-11-08)


### Features

* add i16,i32,i64,i128 types ([3f0751a](https://github.com/sevki/jetstream/commit/3f0751a8588bf0fb5d189865f65417a717f414c4))

## [6.1.0](https://github.com/sevki/jetstream/compare/v6.0.2...v6.1.0) (2024-11-08)


### Features

* start quic server with config ([#159](https://github.com/sevki/jetstream/issues/159)) ([3433981](https://github.com/sevki/jetstream/commit/34339819f26044e0a19c5c668c2fb710734af564))

## [6.0.2](https://github.com/sevki/jetstream/compare/v6.0.1...v6.0.2) (2024-11-08)


### Bug Fixes

* ci/cd ([#157](https://github.com/sevki/jetstream/issues/157)) ([3f7ff1e](https://github.com/sevki/jetstream/commit/3f7ff1ea0b561136bc51c65f1acc0f683cfafb01))

## [6.0.1](https://github.com/sevki/jetstream/compare/v6.0.0...v6.0.1) (2024-11-08)


### Bug Fixes

* trait_variant use ([#155](https://github.com/sevki/jetstream/issues/155)) ([3cd5665](https://github.com/sevki/jetstream/commit/3cd5665454bb42c3e38606c4a3dd9355ef63fc9b))

## [6.0.0](https://github.com/sevki/jetstream/compare/v5.4.2...v6.0.0) (2024-11-07)


### ⚠ BREAKING CHANGES

* splits up packages
* move modules to sensible parents
* protocol -> coding
* merge all the creates

### Features

* autopub ([73a0844](https://github.com/sevki/jetstream/commit/73a0844e9a7fcc55bf39b39325587d237c549a6e))
* hide filesystem behind a feautre-flag ([9aa880d](https://github.com/sevki/jetstream/commit/9aa880de8d51c88e64d08248f47ddf1d0137db98))
* **macros:** service macro to remove boilerplate code ([e0a9295](https://github.com/sevki/jetstream/commit/e0a9295674327b5eea96922c3054d0e3be07c4a4))
* modularize components ([7262a66](https://github.com/sevki/jetstream/commit/7262a6665993d0d7705191717d773fadcac5173a))
* release please ([7d7bedd](https://github.com/sevki/jetstream/commit/7d7beddcae75613433076a9f77156989b2de1f47))
* release please ([044cceb](https://github.com/sevki/jetstream/commit/044cceb76e544e8c315b6e1d33a321795280e847))
* revamp service ([#147](https://github.com/sevki/jetstream/issues/147)) ([6d96be8](https://github.com/sevki/jetstream/commit/6d96be89affc75b9e534122ac9305257861be706))
* rust-clippy code scanning ([3dfb39f](https://github.com/sevki/jetstream/commit/3dfb39f1c4c9c887931a1686a7c1208fa1182e18))
* use sccache ([#142](https://github.com/sevki/jetstream/issues/142)) ([89f96ab](https://github.com/sevki/jetstream/commit/89f96abf5b0527d45bf75e00fafd4cd197fff1d6))
* use serde_bytes::ByteBuf instead of Bytes ([a1101d9](https://github.com/sevki/jetstream/commit/a1101d99fcfc60ddff2314cebd99a56da706cf7b))
* virtio support ([ce13217](https://github.com/sevki/jetstream/commit/ce13217e4429270226ef43661acab21619493351))
* **wireformat:** add u128 ([c76f6c4](https://github.com/sevki/jetstream/commit/c76f6c4c64fe57dd5948e522c8d68b8114b607ab))


### Bug Fixes

* auto-release ([964036c](https://github.com/sevki/jetstream/commit/964036cd97bfbae055f3b52653387d65524f1972))
* auto-release feature ([6505b0f](https://github.com/sevki/jetstream/commit/6505b0ff66ce16b1032efe722620d03fbe945769))
* bothced update ([b3b7003](https://github.com/sevki/jetstream/commit/b3b7003f565fc833804a70519aeb0741d03f34be))
* broken release-please ([089bb22](https://github.com/sevki/jetstream/commit/089bb2277ba7025b61dbff32473d9b4b1836acd9))
* bump zero copy ([#140](https://github.com/sevki/jetstream/issues/140)) ([4bb933f](https://github.com/sevki/jetstream/commit/4bb933f8ab6399e44e9a9ba4dc9ff92b43d69454))
* ci release-please ([de391e5](https://github.com/sevki/jetstream/commit/de391e58d30f5f08d89f2f9251e9f734bd945bb1))
* filesystem under feature flag, rm newline ([de4cf79](https://github.com/sevki/jetstream/commit/de4cf791a89b794dbfcb48325b1a90f26e421616))
* ignore e2e tests ([e066dde](https://github.com/sevki/jetstream/commit/e066dde3f735c2524118ee0e8128555775d0eeb7))
* lint errors ([4f50d0b](https://github.com/sevki/jetstream/commit/4f50d0b73ddcfeaee2e86a2d27d061d87a0c1134))
* **macros:** protocol macro fix to spread fn args ([b261a28](https://github.com/sevki/jetstream/commit/b261a286e033e2f9ba1462dc8a5dd06adf4e5ca3))
* make data io::Read ([12a864e](https://github.com/sevki/jetstream/commit/12a864e93e402c0145ff1f206589164bc920fbdd))
* make data io::Read ([910c75a](https://github.com/sevki/jetstream/commit/910c75ad120d6b699691dc194377138837b2a8f5))
* make data io::Read ([77b3680](https://github.com/sevki/jetstream/commit/77b3680ff8ac312b14303f864d92bb763659b64f))
* readme ([dc77722](https://github.com/sevki/jetstream/commit/dc777223fe95ea79866320b7831e4e3c89631820))
* release again ([3a6e65e](https://github.com/sevki/jetstream/commit/3a6e65e2d8181bcc93a41a577e2ca04c9ca70bf2))
* release workflow ([4abeb24](https://github.com/sevki/jetstream/commit/4abeb24e99aec6ae9ddcc083136cc765b84ba094))
* revert serde_bytes to bytes with serde ([2e02460](https://github.com/sevki/jetstream/commit/2e02460c71890a59af315f29897d4edbe79f9c54))
* unused git ([3fe908e](https://github.com/sevki/jetstream/commit/3fe908e405bc870a8089b18e88dae245f41eeb68))
* Update client_tests.rs ([4c50132](https://github.com/sevki/jetstream/commit/4c50132ba0d6afa46f849db7d0fa356e64947653))
* update release flow ([#144](https://github.com/sevki/jetstream/issues/144)) ([36dd4af](https://github.com/sevki/jetstream/commit/36dd4af7686d405eeaa42d53e7eaac934d901489))
* Update release.yml ([b058b38](https://github.com/sevki/jetstream/commit/b058b38367db97630b91af82e6bb11e26728708e))
* Update release.yml ([#122](https://github.com/sevki/jetstream/issues/122)) ([566fe1f](https://github.com/sevki/jetstream/commit/566fe1f64c6d6f25d157a227b261c69c9b3e83b5))
* update to v2 upload sarif ([e38bacb](https://github.com/sevki/jetstream/commit/e38bacb216db9b6560b3fe0e284b54425b4e251e))
* version ([822bf0e](https://github.com/sevki/jetstream/commit/822bf0ed14786912a6ca9a7329a577f6d2367945))
* warnings ([62d8013](https://github.com/sevki/jetstream/commit/62d8013e3b9fa2c0120cd9269894ed640c03fb83))
* **wireformat:** from_bytes doesn't require a mutable buf ([437c35c](https://github.com/sevki/jetstream/commit/437c35c2229cd5980f752573fda43c1d6dfae725))


### Code Refactoring

* merge all the creates ([faa0a1a](https://github.com/sevki/jetstream/commit/faa0a1a1194bac41d8e05efd0108e0c1821fa543))
* move modules to sensible parents ([4eba5fb](https://github.com/sevki/jetstream/commit/4eba5fb0105626fd56c42f2062c7cfbe7293279b))
* protocol -&gt; coding ([5f86bc7](https://github.com/sevki/jetstream/commit/5f86bc78a85728091f8411ab00f5ad3e4a960df2))

## [5.4.2](https://github.com/sevki/jetstream/compare/v5.4.1...v5.4.2) (2024-11-07)


### Bug Fixes

* auto-release ([964036c](https://github.com/sevki/jetstream/commit/964036cd97bfbae055f3b52653387d65524f1972))

## [5.4.2](https://github.com/sevki/jetstream/compare/v5.4.1...v5.4.2) (2024-11-07)


### Bug Fixes

* auto-release ([964036c](https://github.com/sevki/jetstream/commit/964036cd97bfbae055f3b52653387d65524f1972))

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
