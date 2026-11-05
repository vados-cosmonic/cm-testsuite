# Generated from rust/Cargo.lock by xtask buck-lock. Do not edit.

def pinned_crates():
    native.http_archive(name = "anstream-1.0.0", urls = ["https://static.crates.io/crates/anstream/1.0.0/download"], sha256 = "824a212faf96e9acacdbd09febd34438f8f711fb84e09a8916013cd7815ca28d", strip_prefix = "anstream-1.0.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "anstyle-1.0.14", urls = ["https://static.crates.io/crates/anstyle/1.0.14/download"], sha256 = "940b3a0ca603d1eade50a4846a2afffd5ef57a9feac2c0e2ec2e14f9ead76000", strip_prefix = "anstyle-1.0.14", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "anstyle-parse-1.0.0", urls = ["https://static.crates.io/crates/anstyle-parse/1.0.0/download"], sha256 = "52ce7f38b242319f7cabaa6813055467063ecdc9d355bbb4ce0c68908cd8130e", strip_prefix = "anstyle-parse-1.0.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "anstyle-query-1.1.5", urls = ["https://static.crates.io/crates/anstyle-query/1.1.5/download"], sha256 = "40c48f72fd53cd289104fc64099abca73db4166ad86ea0b4341abe65af83dadc", strip_prefix = "anstyle-query-1.1.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "anstyle-wincon-3.0.11", urls = ["https://static.crates.io/crates/anstyle-wincon/3.0.11/download"], sha256 = "291e6a250ff86cd4a820112fb8898808a366d8f9f58ce16d1f538353ad55747d", strip_prefix = "anstyle-wincon-3.0.11", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "anyhow-1.0.103", urls = ["https://static.crates.io/crates/anyhow/1.0.103/download"], sha256 = "2a4385e2e34eb35d6b3efe798b9eb88096925d87726c0798709bf56d9ed84af3", strip_prefix = "anyhow-1.0.103", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "base64-0.22.1", urls = ["https://static.crates.io/crates/base64/0.22.1/download"], sha256 = "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6", strip_prefix = "base64-0.22.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "base64-0.23.1", urls = ["https://static.crates.io/crates/base64/0.23.1/download"], sha256 = "ac07cdecf99051d9a5238b80f35af32cdeba5b336e55d957b318b50137e18da5", strip_prefix = "base64-0.23.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "bitflags-2.13.2", urls = ["https://static.crates.io/crates/bitflags/2.13.2/download"], sha256 = "3ded4057c258ba199e2d26386d3af3780957ecaee6c4ef4041c6b4b8b97c0b06", strip_prefix = "bitflags-2.13.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "block-buffer-0.10.4", urls = ["https://static.crates.io/crates/block-buffer/0.10.4/download"], sha256 = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71", strip_prefix = "block-buffer-0.10.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "bumpalo-3.20.3", urls = ["https://static.crates.io/crates/bumpalo/3.20.3/download"], sha256 = "72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649", strip_prefix = "bumpalo-3.20.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "bytes-1.12.1", urls = ["https://static.crates.io/crates/bytes/1.12.1/download"], sha256 = "fc652a48c352aef3ea3aed32080501cf3ef6ed5da78602a020c991775b0aff04", strip_prefix = "bytes-1.12.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "cc-1.11.0", urls = ["https://static.crates.io/crates/cc/1.11.0/download"], sha256 = "375989decec1a0fa424442bed520fa12ede43a48475ce8c7417d2c69b32b5f6b", strip_prefix = "cc-1.11.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "cfg-if-1.0.5", urls = ["https://static.crates.io/crates/cfg-if/1.0.5/download"], sha256 = "4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600", strip_prefix = "cfg-if-1.0.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "clap-4.6.7", urls = ["https://static.crates.io/crates/clap/4.6.7/download"], sha256 = "aa8876b300ab35ba921adea3dfd70157a46249b33f95c9084ae5709785478946", strip_prefix = "clap-4.6.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "clap_builder-4.6.7", urls = ["https://static.crates.io/crates/clap_builder/4.6.7/download"], sha256 = "ec0797fb7aeb1406c84efac526901f7ec3ead2124f946b494e72879d4b54704d", strip_prefix = "clap_builder-4.6.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "clap_derive-4.6.7", urls = ["https://static.crates.io/crates/clap_derive/4.6.7/download"], sha256 = "f9c751b79415d4e559e3d1fcf128e09e720eb673a06d26cf6f392d37d75b66e0", strip_prefix = "clap_derive-4.6.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "clap_lex-1.1.1", urls = ["https://static.crates.io/crates/clap_lex/1.1.1/download"], sha256 = "1c133bc6a41be0d194c306b5506d15e6feeea7b1d6604bd3f8310dfb2ca96486", strip_prefix = "clap_lex-1.1.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "colorchoice-1.0.5", urls = ["https://static.crates.io/crates/colorchoice/1.0.5/download"], sha256 = "1d07550c9036bf2ae0c684c4297d503f838287c83c53686d05370d0e139ae570", strip_prefix = "colorchoice-1.0.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "cookie-0.18.2", urls = ["https://static.crates.io/crates/cookie/0.18.2/download"], sha256 = "1a373e3602691c3cdea496d2f0ee5935151e6168fe87739483c463db1b2f2f87", strip_prefix = "cookie-0.18.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "cookie_store-0.22.1", urls = ["https://static.crates.io/crates/cookie_store/0.22.1/download"], sha256 = "15b2c103cf610ec6cae3da84a766285b42fd16aad564758459e6ecf128c75206", strip_prefix = "cookie_store-0.22.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "cpufeatures-0.2.17", urls = ["https://static.crates.io/crates/cpufeatures/0.2.17/download"], sha256 = "59ed5838eebb26a2bb2e58f6d5b5316989ae9d08bab10e0e6d103e656d1b0280", strip_prefix = "cpufeatures-0.2.17", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "crypto-common-0.1.7", urls = ["https://static.crates.io/crates/crypto-common/0.1.7/download"], sha256 = "78c8292055d1c1df0cce5d180393dc8cce0abec0a7102adb6c7b1eef6016d60a", strip_prefix = "crypto-common-0.1.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "deranged-0.5.9", urls = ["https://static.crates.io/crates/deranged/0.5.9/download"], sha256 = "9e9de72ce2ad1f90dc62fa25f0f430ef85eb4b0d8fa0be4f30373bc40a21d28e", strip_prefix = "deranged-0.5.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "digest-0.10.7", urls = ["https://static.crates.io/crates/digest/0.10.7/download"], sha256 = "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292", strip_prefix = "digest-0.10.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "displaydoc-0.2.7", urls = ["https://static.crates.io/crates/displaydoc/0.2.7/download"], sha256 = "c6232dd377dcc64799954cbd3a9bb882e9cdc1308ccd87b1c098f1fb2eaf82a8", strip_prefix = "displaydoc-0.2.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "document-features-0.2.12", urls = ["https://static.crates.io/crates/document-features/0.2.12/download"], sha256 = "d4b8a88685455ed29a21542a33abd9cb6510b6b129abadabdcef0f4c55bc8f61", strip_prefix = "document-features-0.2.12", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "equivalent-1.0.3", urls = ["https://static.crates.io/crates/equivalent/1.0.3/download"], sha256 = "00d174d5400e5e8fd687ad1049e2f578285fa914201b1af7e8b112a4546bd826", strip_prefix = "equivalent-1.0.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "errno-0.3.14", urls = ["https://static.crates.io/crates/errno/0.3.14/download"], sha256 = "39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb", strip_prefix = "errno-0.3.14", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "fastrand-2.5.0", urls = ["https://static.crates.io/crates/fastrand/2.5.0/download"], sha256 = "da7c62ceae207dd37ea5b845da6a0696c799f85e97da1ab5b7910be3c1c80223", strip_prefix = "fastrand-2.5.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "find-msvc-tools-0.1.15", urls = ["https://static.crates.io/crates/find-msvc-tools/0.1.15/download"], sha256 = "1c66915cc5391097de7a2b0b972e8505aa33c0df401fd3a6811dbdb32881ba48", strip_prefix = "find-msvc-tools-0.1.15", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "form_urlencoded-1.2.2", urls = ["https://static.crates.io/crates/form_urlencoded/1.2.2/download"], sha256 = "cb4cb245038516f5f85277875cdaa4f7d2c9a0fa0468de06ed190163b1581fcf", strip_prefix = "form_urlencoded-1.2.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "generic-array-0.14.7", urls = ["https://static.crates.io/crates/generic-array/0.14.7/download"], sha256 = "85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a", strip_prefix = "generic-array-0.14.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "getrandom-0.2.17", urls = ["https://static.crates.io/crates/getrandom/0.2.17/download"], sha256 = "ff2abc00be7fca6ebc474524697ae276ad847ad0a6b3faa4bcb027e9a4614ad0", strip_prefix = "getrandom-0.2.17", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "getrandom-0.4.3", urls = ["https://static.crates.io/crates/getrandom/0.4.3/download"], sha256 = "300e883d756b2e4ec94e02791f39b04b522276138852cfc41d9fb7e904106099", strip_prefix = "getrandom-0.4.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "hashbrown-0.17.1", urls = ["https://static.crates.io/crates/hashbrown/0.17.1/download"], sha256 = "ed5909b6e89a2db4456e54cd5f673791d7eca6732202bbf2a9cc504fe2f9b84a", strip_prefix = "hashbrown-0.17.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "heck-0.5.0", urls = ["https://static.crates.io/crates/heck/0.5.0/download"], sha256 = "2304e00983f87ffb38b55b444b5e3b60a884b5d30c0fca7d82fe33449bbe55ea", strip_prefix = "heck-0.5.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "http-1.5.0", urls = ["https://static.crates.io/crates/http/1.5.0/download"], sha256 = "918d3568bebf352712bc2ef3d46a8bcf1a75b373be6539de198e9105cbbf9ce0", strip_prefix = "http-1.5.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "httparse-1.10.1", urls = ["https://static.crates.io/crates/httparse/1.10.1/download"], sha256 = "6dbf3de79e51f3d586ab4cb9d5c3e2c14aa28ed23d180cf89b4df0454a69cc87", strip_prefix = "httparse-1.10.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_collections-2.4.0", urls = ["https://static.crates.io/crates/icu_collections/2.4.0/download"], sha256 = "fe090a42a0d3140fed8c63263ed1fecc02422cb2f4bfe55cd8de2de1b8e105dd", strip_prefix = "icu_collections-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_locale_core-2.4.0", urls = ["https://static.crates.io/crates/icu_locale_core/2.4.0/download"], sha256 = "25d29b57c17c6528d0dc4c2141cfa2d08200465acb537a3909c8b9b8a4a984c0", strip_prefix = "icu_locale_core-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_normalizer-2.4.0", urls = ["https://static.crates.io/crates/icu_normalizer/2.4.0/download"], sha256 = "404a3b8c486f205c87fe7f23ab6894d84a4ad76d6e4671ae1981ba7def87c599", strip_prefix = "icu_normalizer-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_normalizer_data-2.4.0", urls = ["https://static.crates.io/crates/icu_normalizer_data/2.4.0/download"], sha256 = "20f0088bfb7441f7a37fa0c9e20cb1e9cba1cac6b9ce8cd2a6dc44294f8e8cbc", strip_prefix = "icu_normalizer_data-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_properties-2.4.0", urls = ["https://static.crates.io/crates/icu_properties/2.4.0/download"], sha256 = "2ea83d991c56158f7cf898bf86436c1dfe7c07ee82fe88951f2695c9f37a379f", strip_prefix = "icu_properties-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_properties_data-2.4.0", urls = ["https://static.crates.io/crates/icu_properties_data/2.4.0/download"], sha256 = "b98f29af82d02b163fe50161388350e743bb3107cc8e8c2f7314cb505faddfcf", strip_prefix = "icu_properties_data-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "icu_provider-2.4.0", urls = ["https://static.crates.io/crates/icu_provider/2.4.0/download"], sha256 = "413099315636c588ee8f011190b329b971a09a8fae9d157e01dcae8379468514", strip_prefix = "icu_provider-2.4.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "idna-1.1.0", urls = ["https://static.crates.io/crates/idna/1.1.0/download"], sha256 = "3b0875f23caa03898994f6ddc501886a45c7d3d62d04d2d90788d47be1b1e4de", strip_prefix = "idna-1.1.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "idna_adapter-1.2.2", urls = ["https://static.crates.io/crates/idna_adapter/1.2.2/download"], sha256 = "cb68373c0d6620ef8105e855e7745e18b0d00d3bdb07fb532e434244cdb9a714", strip_prefix = "idna_adapter-1.2.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "indexmap-2.14.2", urls = ["https://static.crates.io/crates/indexmap/2.14.2/download"], sha256 = "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855", strip_prefix = "indexmap-2.14.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "is_terminal_polyfill-1.70.2", urls = ["https://static.crates.io/crates/is_terminal_polyfill/1.70.2/download"], sha256 = "a6cb138bb79a146c1bd460005623e142ef0181e3d0219cb493e02f7d08a35695", strip_prefix = "is_terminal_polyfill-1.70.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "itoa-1.0.18", urls = ["https://static.crates.io/crates/itoa/1.0.18/download"], sha256 = "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682", strip_prefix = "itoa-1.0.18", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "leb128fmt-0.1.0", urls = ["https://static.crates.io/crates/leb128fmt/0.1.0/download"], sha256 = "09edd9e8b54e49e587e4f6295a7d29c3ea94d469cb40ab8ca70b288248a81db2", strip_prefix = "leb128fmt-0.1.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "libc-0.2.191", urls = ["https://static.crates.io/crates/libc/0.2.191/download"], sha256 = "65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a", strip_prefix = "libc-0.2.191", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "linux-raw-sys-0.12.1", urls = ["https://static.crates.io/crates/linux-raw-sys/0.12.1/download"], sha256 = "32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53", strip_prefix = "linux-raw-sys-0.12.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "litemap-0.8.3", urls = ["https://static.crates.io/crates/litemap/0.8.3/download"], sha256 = "47d9d19d1d6efa0109d2f65ff4c85cddd50bd572e5a00127ab10987290bcefae", strip_prefix = "litemap-0.8.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "litrs-1.0.1", urls = ["https://static.crates.io/crates/litrs/1.0.1/download"], sha256 = "4744e383959f0db86ede514b809b1c53251889093803c05267acc7d4e7030d70", strip_prefix = "litrs-1.0.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "log-0.4.35", urls = ["https://static.crates.io/crates/log/0.4.35/download"], sha256 = "5d15f86d12da44cd8d4ec3c73984537eb7f4cadcb9776a575a0a8a05ca11005c", strip_prefix = "log-0.4.35", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "memchr-2.8.3", urls = ["https://static.crates.io/crates/memchr/2.8.3/download"], sha256 = "cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98", strip_prefix = "memchr-2.8.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "num-conv-0.2.2", urls = ["https://static.crates.io/crates/num-conv/0.2.2/download"], sha256 = "521739c6d2bac4aa25192232afe6841231376b2b26d4d9fae5ecf8ca5772e441", strip_prefix = "num-conv-0.2.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "once_cell-1.21.4", urls = ["https://static.crates.io/crates/once_cell/1.21.4/download"], sha256 = "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50", strip_prefix = "once_cell-1.21.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "once_cell_polyfill-1.70.2", urls = ["https://static.crates.io/crates/once_cell_polyfill/1.70.2/download"], sha256 = "384b8ab6d37215f3c5301a95a4accb5d64aa607f1fcb26a11b5303878451b4fe", strip_prefix = "once_cell_polyfill-1.70.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "percent-encoding-2.3.2", urls = ["https://static.crates.io/crates/percent-encoding/2.3.2/download"], sha256 = "9b4f627cb1b25917193a259e49bdad08f671f8d9708acfd5fe0a8c1455d87220", strip_prefix = "percent-encoding-2.3.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "potential_utf-0.1.6", urls = ["https://static.crates.io/crates/potential_utf/0.1.6/download"], sha256 = "d83eb9bc6d8e5cf568e7a1101d60ee05e81ed50ea106026f3d18deeb046d7661", strip_prefix = "potential_utf-0.1.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "powerfmt-0.2.1", urls = ["https://static.crates.io/crates/powerfmt/0.2.1/download"], sha256 = "4a6394b9e965e73d0a289ee54f589087e2c676aedf60885baf52c76b771e4958", strip_prefix = "powerfmt-0.2.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "proc-macro2-1.0.107", urls = ["https://static.crates.io/crates/proc-macro2/1.0.107/download"], sha256 = "985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9", strip_prefix = "proc-macro2-1.0.107", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "quote-1.0.47", urls = ["https://static.crates.io/crates/quote/1.0.47/download"], sha256 = "1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001", strip_prefix = "quote-1.0.47", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "r-efi-6.0.0", urls = ["https://static.crates.io/crates/r-efi/6.0.0/download"], sha256 = "f8dcc9c7d52a811697d2151c701e0d08956f92b0e24136cf4cf27b57a6a0d9bf", strip_prefix = "r-efi-6.0.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "ring-0.17.14", urls = ["https://static.crates.io/crates/ring/0.17.14/download"], sha256 = "a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7", strip_prefix = "ring-0.17.14", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "rustix-1.1.6", urls = ["https://static.crates.io/crates/rustix/1.1.6/download"], sha256 = "e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa", strip_prefix = "rustix-1.1.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "rustls-0.23.46", urls = ["https://static.crates.io/crates/rustls/0.23.46/download"], sha256 = "48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf", strip_prefix = "rustls-0.23.46", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "rustls-pki-types-1.15.1", urls = ["https://static.crates.io/crates/rustls-pki-types/1.15.1/download"], sha256 = "2f4925028c7eb5d1fcdaf196971378ed9d2c1c4efc7dc5d011256f76c99c0a96", strip_prefix = "rustls-pki-types-1.15.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "rustls-webpki-0.103.16", urls = ["https://static.crates.io/crates/rustls-webpki/0.103.16/download"], sha256 = "4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281", strip_prefix = "rustls-webpki-0.103.16", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "semver-1.0.28", urls = ["https://static.crates.io/crates/semver/1.0.28/download"], sha256 = "8a7852d02fc848982e0c167ef163aaff9cd91dc640ba85e263cb1ce46fae51cd", strip_prefix = "semver-1.0.28", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "serde-1.0.228", urls = ["https://static.crates.io/crates/serde/1.0.228/download"], sha256 = "9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e", strip_prefix = "serde-1.0.228", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "serde_core-1.0.228", urls = ["https://static.crates.io/crates/serde_core/1.0.228/download"], sha256 = "41d385c7d4ca58e59fc732af25c3983b67ac852c1a25000afe1175de458b67ad", strip_prefix = "serde_core-1.0.228", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "serde_derive-1.0.228", urls = ["https://static.crates.io/crates/serde_derive/1.0.228/download"], sha256 = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79", strip_prefix = "serde_derive-1.0.228", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "serde_json-1.0.150", urls = ["https://static.crates.io/crates/serde_json/1.0.150/download"], sha256 = "e8014e44b4736ed0538adeecded0fce2a272f22dc9578a7eb6b2d9993c74cfb9", strip_prefix = "serde_json-1.0.150", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "serde_spanned-0.6.9", urls = ["https://static.crates.io/crates/serde_spanned/0.6.9/download"], sha256 = "bf41e0cfaf7226dca15e8197172c295a782857fcb97fad1808a166870dee75a3", strip_prefix = "serde_spanned-0.6.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "sha2-0.10.9", urls = ["https://static.crates.io/crates/sha2/0.10.9/download"], sha256 = "a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283", strip_prefix = "sha2-0.10.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "shlex-2.0.1", urls = ["https://static.crates.io/crates/shlex/2.0.1/download"], sha256 = "f8fadd59c855ef2080decdef8ff161eb6661b86933c9d82e5ba29dc602a55aba", strip_prefix = "shlex-2.0.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "smallvec-1.16.4", urls = ["https://static.crates.io/crates/smallvec/1.16.4/download"], sha256 = "d0d5d4b1981b8bbbb7aed204cae5726d683d5d6ac7e74ec54122ea28f315e7c8", strip_prefix = "smallvec-1.16.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "stable_deref_trait-1.2.1", urls = ["https://static.crates.io/crates/stable_deref_trait/1.2.1/download"], sha256 = "6ce2be8dc25455e1f91df71bfa12ad37d7af1092ae736f3a6cd0e37bc7810596", strip_prefix = "stable_deref_trait-1.2.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "strsim-0.11.1", urls = ["https://static.crates.io/crates/strsim/0.11.1/download"], sha256 = "7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f", strip_prefix = "strsim-0.11.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "subtle-2.6.1", urls = ["https://static.crates.io/crates/subtle/2.6.1/download"], sha256 = "13c2bddecc57b384dee18652358fb23172facb8a2c51ccc10d74c157bdea3292", strip_prefix = "subtle-2.6.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "syn-2.0.119", urls = ["https://static.crates.io/crates/syn/2.0.119/download"], sha256 = "872831b642d1a07999a962a351ed35b955ea2cfc8f3862091e2a240a84f17297", strip_prefix = "syn-2.0.119", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "syn-3.0.9", urls = ["https://static.crates.io/crates/syn/3.0.9/download"], sha256 = "d78c8dee4c7bf0e14673097256fed6142ce9d3b85a408189d07482442145823b", strip_prefix = "syn-3.0.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "synstructure-0.14.0", urls = ["https://static.crates.io/crates/synstructure/0.14.0/download"], sha256 = "901704edd0dfe137f1987838ee4f259e4e063c31371bdb423f7ae38ec6f77f02", strip_prefix = "synstructure-0.14.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "tempfile-3.27.0", urls = ["https://static.crates.io/crates/tempfile/3.27.0/download"], sha256 = "32497e9a4c7b38532efcdebeef879707aa9f794296a4f0244f6f69e9bc8574bd", strip_prefix = "tempfile-3.27.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "time-0.3.55", urls = ["https://static.crates.io/crates/time/0.3.55/download"], sha256 = "cdb87b95ec50ddfa440816d227a17b2ccbdda963a316a727fda0fc4334f7d134", strip_prefix = "time-0.3.55", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "time-core-0.1.9", urls = ["https://static.crates.io/crates/time-core/0.1.9/download"], sha256 = "9e1c906769ad99c88eaa54e728060edef082f8e358ff32030cb7c7d315e81109", strip_prefix = "time-core-0.1.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "time-macros-0.2.32", urls = ["https://static.crates.io/crates/time-macros/0.2.32/download"], sha256 = "7e689342a48d2ea927c87ea50cabf8594854bf940e9310208848d680d668ed85", strip_prefix = "time-macros-0.2.32", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "tinystr-0.8.5", urls = ["https://static.crates.io/crates/tinystr/0.8.5/download"], sha256 = "abc840c238154d58f9e86b2792261edfde565a17696e1cda1a449de27979801a", strip_prefix = "tinystr-0.8.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "toml-0.8.23", urls = ["https://static.crates.io/crates/toml/0.8.23/download"], sha256 = "dc1beb996b9d83529a9e75c17a1686767d148d70663143c7854d8b4a09ced362", strip_prefix = "toml-0.8.23", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "toml_datetime-0.6.11", urls = ["https://static.crates.io/crates/toml_datetime/0.6.11/download"], sha256 = "22cddaf88f4fbc13c51aebbf5f8eceb5c7c5a9da2ac40a13519eb5b0a0e8f11c", strip_prefix = "toml_datetime-0.6.11", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "toml_edit-0.22.27", urls = ["https://static.crates.io/crates/toml_edit/0.22.27/download"], sha256 = "41fe8c660ae4257887cf66394862d21dbca4a6ddd26f04a3560410406a2f819a", strip_prefix = "toml_edit-0.22.27", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "toml_write-0.1.2", urls = ["https://static.crates.io/crates/toml_write/0.1.2/download"], sha256 = "5d99f8c9a7727884afe522e9bd5edbfc91a3312b36a77b5fb8926e4c31a41801", strip_prefix = "toml_write-0.1.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "typenum-1.20.1", urls = ["https://static.crates.io/crates/typenum/1.20.1/download"], sha256 = "b6f5e870be6c3b371b77fe0ee0bafb859fa4964b4404c27de1d380043c4dda20", strip_prefix = "typenum-1.20.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "unicode-ident-1.0.27", urls = ["https://static.crates.io/crates/unicode-ident/1.0.27/download"], sha256 = "a2c754d6c33795a1c324727428e5a7dedb5b06195f9890bdbcba760d3e246563", strip_prefix = "unicode-ident-1.0.27", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "unicode-width-0.2.3", urls = ["https://static.crates.io/crates/unicode-width/0.2.3/download"], sha256 = "ca718471876992059f251e33aafc334346ada08042b7661dd1fe313a9bced557", strip_prefix = "unicode-width-0.2.3", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "untrusted-0.9.0", urls = ["https://static.crates.io/crates/untrusted/0.9.0/download"], sha256 = "8ecb6da28b8a351d773b68d5825ac39017e680750f980f3a1a85cd8dd28a47c1", strip_prefix = "untrusted-0.9.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "ureq-3.3.0", urls = ["https://static.crates.io/crates/ureq/3.3.0/download"], sha256 = "dea7109cdcd5864d4eeb1b58a1648dc9bf520360d7af16ec26d0a9354bafcfc0", strip_prefix = "ureq-3.3.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "ureq-proto-0.6.5", urls = ["https://static.crates.io/crates/ureq-proto/0.6.5/download"], sha256 = "841583fc36089d7d59419568ae157f43a5a03a2e5a81ba87c77d1596ef46e922", strip_prefix = "ureq-proto-0.6.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "url-2.5.8", urls = ["https://static.crates.io/crates/url/2.5.8/download"], sha256 = "ff67a8a4397373c3ef660812acab3268222035010ab8680ec4215f38ba3d0eed", strip_prefix = "url-2.5.8", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "utf8-zero-0.8.1", urls = ["https://static.crates.io/crates/utf8-zero/0.8.1/download"], sha256 = "b8c0a043c9540bae7c578c88f91dda8bd82e59ae27c21baca69c8b191aaf5a6e", strip_prefix = "utf8-zero-0.8.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "utf8_iter-1.1.0", urls = ["https://static.crates.io/crates/utf8_iter/1.1.0/download"], sha256 = "af3bd9d3e571c26fcce9d033b87ea5ab061b8d2ffe9b7a5675d810d68f03edf0", strip_prefix = "utf8_iter-1.1.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "utf8parse-0.2.2", urls = ["https://static.crates.io/crates/utf8parse/0.2.2/download"], sha256 = "06abde3611657adf66d383f00b093d7faecc7fa57071cce2578660c9f1010821", strip_prefix = "utf8parse-0.2.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "utf_types-1.0.0", urls = ["https://static.crates.io/crates/utf_types/1.0.0/download"], sha256 = "3a3b425286980a367278453f3b6d28cdcb134b574e5a35824a6ff5eebad50bb6", strip_prefix = "utf_types-1.0.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "version_check-0.9.5", urls = ["https://static.crates.io/crates/version_check/0.9.5/download"], sha256 = "0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a", strip_prefix = "version_check-0.9.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "wasi-0.11.1_wasi-snapshot-preview1", urls = ["https://static.crates.io/crates/wasi/0.11.1+wasi-snapshot-preview1/download"], sha256 = "ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b", strip_prefix = "wasi-0.11.1+wasi-snapshot-preview1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "wasm-encoder-0.263.0", urls = ["https://static.crates.io/crates/wasm-encoder/0.263.0/download"], sha256 = "0b883a26aea02eaabe2d2c3fba7eb039ef5cdcb5d5201ae5551e73d5e251a81f", strip_prefix = "wasm-encoder-0.263.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "wasmparser-0.263.0", urls = ["https://static.crates.io/crates/wasmparser/0.263.0/download"], sha256 = "4f5fa0a054ae153606df820eb0a84213e28cda41bbe04ba2628e71b6ba100fcb", strip_prefix = "wasmparser-0.263.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "wast-263.0.0", urls = ["https://static.crates.io/crates/wast/263.0.0/download"], sha256 = "69d90bcf581b501aa1c358bc5dc0faa979cd375796908860ebcb6d1d70b907c7", strip_prefix = "wast-263.0.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "webpki-roots-1.0.11", urls = ["https://static.crates.io/crates/webpki-roots/1.0.11/download"], sha256 = "f5f280bbb2b287bc8e7de7e3852945a0ff50d1bbcb5ed42b6c7a568c546df171", strip_prefix = "webpki-roots-1.0.11", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows-link-0.2.1", urls = ["https://static.crates.io/crates/windows-link/0.2.1/download"], sha256 = "f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5", strip_prefix = "windows-link-0.2.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows-sys-0.52.0", urls = ["https://static.crates.io/crates/windows-sys/0.52.0/download"], sha256 = "282be5f36a8ce781fad8c8ae18fa3f9beff57ec1b52cb3de0789201425d9a33d", strip_prefix = "windows-sys-0.52.0", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows-sys-0.61.2", urls = ["https://static.crates.io/crates/windows-sys/0.61.2/download"], sha256 = "ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc", strip_prefix = "windows-sys-0.61.2", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows-targets-0.52.6", urls = ["https://static.crates.io/crates/windows-targets/0.52.6/download"], sha256 = "9b724f72796e036ab90c1021d4780d4d3d648aca59e491e6b98e725b84e99973", strip_prefix = "windows-targets-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_aarch64_gnullvm-0.52.6", urls = ["https://static.crates.io/crates/windows_aarch64_gnullvm/0.52.6/download"], sha256 = "32a4622180e7a0ec044bb555404c800bc9fd9ec262ec147edd5989ccd0c02cd3", strip_prefix = "windows_aarch64_gnullvm-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_aarch64_msvc-0.52.6", urls = ["https://static.crates.io/crates/windows_aarch64_msvc/0.52.6/download"], sha256 = "09ec2a7bb152e2252b53fa7803150007879548bc709c039df7627cabbd05d469", strip_prefix = "windows_aarch64_msvc-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_i686_gnu-0.52.6", urls = ["https://static.crates.io/crates/windows_i686_gnu/0.52.6/download"], sha256 = "8e9b5ad5ab802e97eb8e295ac6720e509ee4c243f69d781394014ebfe8bbfa0b", strip_prefix = "windows_i686_gnu-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_i686_gnullvm-0.52.6", urls = ["https://static.crates.io/crates/windows_i686_gnullvm/0.52.6/download"], sha256 = "0eee52d38c090b3caa76c563b86c3a4bd71ef1a819287c19d586d7334ae8ed66", strip_prefix = "windows_i686_gnullvm-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_i686_msvc-0.52.6", urls = ["https://static.crates.io/crates/windows_i686_msvc/0.52.6/download"], sha256 = "240948bc05c5e7c6dabba28bf89d89ffce3e303022809e73deaefe4f6ec56c66", strip_prefix = "windows_i686_msvc-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_x86_64_gnu-0.52.6", urls = ["https://static.crates.io/crates/windows_x86_64_gnu/0.52.6/download"], sha256 = "147a5c80aabfbf0c7d901cb5895d1de30ef2907eb21fbbab29ca94c5b08b1a78", strip_prefix = "windows_x86_64_gnu-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_x86_64_gnullvm-0.52.6", urls = ["https://static.crates.io/crates/windows_x86_64_gnullvm/0.52.6/download"], sha256 = "24d5b23dc417412679681396f2b49f3de8c1473deb516bd34410872eff51ed0d", strip_prefix = "windows_x86_64_gnullvm-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "windows_x86_64_msvc-0.52.6", urls = ["https://static.crates.io/crates/windows_x86_64_msvc/0.52.6/download"], sha256 = "589f6da84c646204747d1270a2a5661ea66ed1cced2631d546fdfb155959f9ec", strip_prefix = "windows_x86_64_msvc-0.52.6", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "winnow-0.7.15", urls = ["https://static.crates.io/crates/winnow/0.7.15/download"], sha256 = "df79d97927682d2fd8adb29682d1140b343be4ac0f08fd68b7765d9c059d3945", strip_prefix = "winnow-0.7.15", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "writeable-0.6.4", urls = ["https://static.crates.io/crates/writeable/0.6.4/download"], sha256 = "3ad82d2a33cdc9674dc7465672f271e096168fcdbe0f799d9e6db8c5892679dc", strip_prefix = "writeable-0.6.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "yoke-0.8.4", urls = ["https://static.crates.io/crates/yoke/0.8.4/download"], sha256 = "417ef3fcf6c300b7dbd25c85531a746931c02ad536f3cf4786ed1e88223340b3", strip_prefix = "yoke-0.8.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "yoke-derive-0.8.4", urls = ["https://static.crates.io/crates/yoke-derive/0.8.4/download"], sha256 = "ec8ebde2db3681e8c9980cc27822030e68752690ddfa9473e739aeb4dbde6d71", strip_prefix = "yoke-derive-0.8.4", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zerofrom-0.1.8", urls = ["https://static.crates.io/crates/zerofrom/0.1.8/download"], sha256 = "0ec05a11813ea801ff6d75110ad09cd0824ddba17dfe17128ea0d5f68e6c5272", strip_prefix = "zerofrom-0.1.8", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zerofrom-derive-0.1.8", urls = ["https://static.crates.io/crates/zerofrom-derive/0.1.8/download"], sha256 = "f75b4683f6c7f45248d4d64056a24298c6281e0993356d7d1b4a1a962ef10d4a", strip_prefix = "zerofrom-derive-0.1.8", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zeroize-1.9.1", urls = ["https://static.crates.io/crates/zeroize/1.9.1/download"], sha256 = "e13084392c5e4bc371903e2935a5eaeed24905a7511356b883835e18a78f6879", strip_prefix = "zeroize-1.9.1", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zerotrie-0.2.5", urls = ["https://static.crates.io/crates/zerotrie/0.2.5/download"], sha256 = "4ea269c3bd32f0a32c321907a2ae912ba6f4649bb0fc764a15627e99a7095a3f", strip_prefix = "zerotrie-0.2.5", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zerovec-0.11.9", urls = ["https://static.crates.io/crates/zerovec/0.11.9/download"], sha256 = "f58a2799cfe9a1670c7ea8edbd52ef8edb93cb37a7f83bf31ba9e35d9dae6a40", strip_prefix = "zerovec-0.11.9", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zerovec-derive-0.11.7", urls = ["https://static.crates.io/crates/zerovec-derive/0.11.7/download"], sha256 = "c3743ddbf47738d929cd1ae2e199d40d0162979dfe31aef8ae646d2bf00bf97a", strip_prefix = "zerovec-derive-0.11.7", type = "tar.gz", visibility = ["PUBLIC"])
    native.http_archive(name = "zmij-1.0.23", urls = ["https://static.crates.io/crates/zmij/1.0.23/download"], sha256 = "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b", strip_prefix = "zmij-1.0.23", type = "tar.gz", visibility = ["PUBLIC"])

CRATES = {
    "anstream-1.0.0": "//third-party:anstream-1.0.0",
    "anstyle-1.0.14": "//third-party:anstyle-1.0.14",
    "anstyle-parse-1.0.0": "//third-party:anstyle-parse-1.0.0",
    "anstyle-query-1.1.5": "//third-party:anstyle-query-1.1.5",
    "anstyle-wincon-3.0.11": "//third-party:anstyle-wincon-3.0.11",
    "anyhow-1.0.103": "//third-party:anyhow-1.0.103",
    "base64-0.22.1": "//third-party:base64-0.22.1",
    "base64-0.23.1": "//third-party:base64-0.23.1",
    "bitflags-2.13.2": "//third-party:bitflags-2.13.2",
    "block-buffer-0.10.4": "//third-party:block-buffer-0.10.4",
    "bumpalo-3.20.3": "//third-party:bumpalo-3.20.3",
    "bytes-1.12.1": "//third-party:bytes-1.12.1",
    "cc-1.11.0": "//third-party:cc-1.11.0",
    "cfg-if-1.0.5": "//third-party:cfg-if-1.0.5",
    "clap-4.6.7": "//third-party:clap-4.6.7",
    "clap_builder-4.6.7": "//third-party:clap_builder-4.6.7",
    "clap_derive-4.6.7": "//third-party:clap_derive-4.6.7",
    "clap_lex-1.1.1": "//third-party:clap_lex-1.1.1",
    "colorchoice-1.0.5": "//third-party:colorchoice-1.0.5",
    "cookie-0.18.2": "//third-party:cookie-0.18.2",
    "cookie_store-0.22.1": "//third-party:cookie_store-0.22.1",
    "cpufeatures-0.2.17": "//third-party:cpufeatures-0.2.17",
    "crypto-common-0.1.7": "//third-party:crypto-common-0.1.7",
    "deranged-0.5.9": "//third-party:deranged-0.5.9",
    "digest-0.10.7": "//third-party:digest-0.10.7",
    "displaydoc-0.2.7": "//third-party:displaydoc-0.2.7",
    "document-features-0.2.12": "//third-party:document-features-0.2.12",
    "equivalent-1.0.3": "//third-party:equivalent-1.0.3",
    "errno-0.3.14": "//third-party:errno-0.3.14",
    "fastrand-2.5.0": "//third-party:fastrand-2.5.0",
    "find-msvc-tools-0.1.15": "//third-party:find-msvc-tools-0.1.15",
    "form_urlencoded-1.2.2": "//third-party:form_urlencoded-1.2.2",
    "generic-array-0.14.7": "//third-party:generic-array-0.14.7",
    "getrandom-0.2.17": "//third-party:getrandom-0.2.17",
    "getrandom-0.4.3": "//third-party:getrandom-0.4.3",
    "hashbrown-0.17.1": "//third-party:hashbrown-0.17.1",
    "heck-0.5.0": "//third-party:heck-0.5.0",
    "http-1.5.0": "//third-party:http-1.5.0",
    "httparse-1.10.1": "//third-party:httparse-1.10.1",
    "icu_collections-2.4.0": "//third-party:icu_collections-2.4.0",
    "icu_locale_core-2.4.0": "//third-party:icu_locale_core-2.4.0",
    "icu_normalizer-2.4.0": "//third-party:icu_normalizer-2.4.0",
    "icu_normalizer_data-2.4.0": "//third-party:icu_normalizer_data-2.4.0",
    "icu_properties-2.4.0": "//third-party:icu_properties-2.4.0",
    "icu_properties_data-2.4.0": "//third-party:icu_properties_data-2.4.0",
    "icu_provider-2.4.0": "//third-party:icu_provider-2.4.0",
    "idna-1.1.0": "//third-party:idna-1.1.0",
    "idna_adapter-1.2.2": "//third-party:idna_adapter-1.2.2",
    "indexmap-2.14.2": "//third-party:indexmap-2.14.2",
    "is_terminal_polyfill-1.70.2": "//third-party:is_terminal_polyfill-1.70.2",
    "itoa-1.0.18": "//third-party:itoa-1.0.18",
    "leb128fmt-0.1.0": "//third-party:leb128fmt-0.1.0",
    "libc-0.2.191": "//third-party:libc-0.2.191",
    "linux-raw-sys-0.12.1": "//third-party:linux-raw-sys-0.12.1",
    "litemap-0.8.3": "//third-party:litemap-0.8.3",
    "litrs-1.0.1": "//third-party:litrs-1.0.1",
    "log-0.4.35": "//third-party:log-0.4.35",
    "memchr-2.8.3": "//third-party:memchr-2.8.3",
    "num-conv-0.2.2": "//third-party:num-conv-0.2.2",
    "once_cell-1.21.4": "//third-party:once_cell-1.21.4",
    "once_cell_polyfill-1.70.2": "//third-party:once_cell_polyfill-1.70.2",
    "percent-encoding-2.3.2": "//third-party:percent-encoding-2.3.2",
    "potential_utf-0.1.6": "//third-party:potential_utf-0.1.6",
    "powerfmt-0.2.1": "//third-party:powerfmt-0.2.1",
    "proc-macro2-1.0.107": "//third-party:proc-macro2-1.0.107",
    "quote-1.0.47": "//third-party:quote-1.0.47",
    "r-efi-6.0.0": "//third-party:r-efi-6.0.0",
    "ring-0.17.14": "//third-party:ring-0.17.14",
    "rustix-1.1.6": "//third-party:rustix-1.1.6",
    "rustls-0.23.46": "//third-party:rustls-0.23.46",
    "rustls-pki-types-1.15.1": "//third-party:rustls-pki-types-1.15.1",
    "rustls-webpki-0.103.16": "//third-party:rustls-webpki-0.103.16",
    "semver-1.0.28": "//third-party:semver-1.0.28",
    "serde-1.0.228": "//third-party:serde-1.0.228",
    "serde_core-1.0.228": "//third-party:serde_core-1.0.228",
    "serde_derive-1.0.228": "//third-party:serde_derive-1.0.228",
    "serde_json-1.0.150": "//third-party:serde_json-1.0.150",
    "serde_spanned-0.6.9": "//third-party:serde_spanned-0.6.9",
    "sha2-0.10.9": "//third-party:sha2-0.10.9",
    "shlex-2.0.1": "//third-party:shlex-2.0.1",
    "smallvec-1.16.4": "//third-party:smallvec-1.16.4",
    "stable_deref_trait-1.2.1": "//third-party:stable_deref_trait-1.2.1",
    "strsim-0.11.1": "//third-party:strsim-0.11.1",
    "subtle-2.6.1": "//third-party:subtle-2.6.1",
    "syn-2.0.119": "//third-party:syn-2.0.119",
    "syn-3.0.9": "//third-party:syn-3.0.9",
    "synstructure-0.14.0": "//third-party:synstructure-0.14.0",
    "tempfile-3.27.0": "//third-party:tempfile-3.27.0",
    "time-0.3.55": "//third-party:time-0.3.55",
    "time-core-0.1.9": "//third-party:time-core-0.1.9",
    "time-macros-0.2.32": "//third-party:time-macros-0.2.32",
    "tinystr-0.8.5": "//third-party:tinystr-0.8.5",
    "toml-0.8.23": "//third-party:toml-0.8.23",
    "toml_datetime-0.6.11": "//third-party:toml_datetime-0.6.11",
    "toml_edit-0.22.27": "//third-party:toml_edit-0.22.27",
    "toml_write-0.1.2": "//third-party:toml_write-0.1.2",
    "typenum-1.20.1": "//third-party:typenum-1.20.1",
    "unicode-ident-1.0.27": "//third-party:unicode-ident-1.0.27",
    "unicode-width-0.2.3": "//third-party:unicode-width-0.2.3",
    "untrusted-0.9.0": "//third-party:untrusted-0.9.0",
    "ureq-3.3.0": "//third-party:ureq-3.3.0",
    "ureq-proto-0.6.5": "//third-party:ureq-proto-0.6.5",
    "url-2.5.8": "//third-party:url-2.5.8",
    "utf8-zero-0.8.1": "//third-party:utf8-zero-0.8.1",
    "utf8_iter-1.1.0": "//third-party:utf8_iter-1.1.0",
    "utf8parse-0.2.2": "//third-party:utf8parse-0.2.2",
    "utf_types-1.0.0": "//third-party:utf_types-1.0.0",
    "version_check-0.9.5": "//third-party:version_check-0.9.5",
    "wasi-0.11.1+wasi-snapshot-preview1": "//third-party:wasi-0.11.1_wasi-snapshot-preview1",
    "wasm-encoder-0.263.0": "//third-party:wasm-encoder-0.263.0",
    "wasmparser-0.263.0": "//third-party:wasmparser-0.263.0",
    "wast-263.0.0": "//third-party:wast-263.0.0",
    "webpki-roots-1.0.11": "//third-party:webpki-roots-1.0.11",
    "windows-link-0.2.1": "//third-party:windows-link-0.2.1",
    "windows-sys-0.52.0": "//third-party:windows-sys-0.52.0",
    "windows-sys-0.61.2": "//third-party:windows-sys-0.61.2",
    "windows-targets-0.52.6": "//third-party:windows-targets-0.52.6",
    "windows_aarch64_gnullvm-0.52.6": "//third-party:windows_aarch64_gnullvm-0.52.6",
    "windows_aarch64_msvc-0.52.6": "//third-party:windows_aarch64_msvc-0.52.6",
    "windows_i686_gnu-0.52.6": "//third-party:windows_i686_gnu-0.52.6",
    "windows_i686_gnullvm-0.52.6": "//third-party:windows_i686_gnullvm-0.52.6",
    "windows_i686_msvc-0.52.6": "//third-party:windows_i686_msvc-0.52.6",
    "windows_x86_64_gnu-0.52.6": "//third-party:windows_x86_64_gnu-0.52.6",
    "windows_x86_64_gnullvm-0.52.6": "//third-party:windows_x86_64_gnullvm-0.52.6",
    "windows_x86_64_msvc-0.52.6": "//third-party:windows_x86_64_msvc-0.52.6",
    "winnow-0.7.15": "//third-party:winnow-0.7.15",
    "writeable-0.6.4": "//third-party:writeable-0.6.4",
    "yoke-0.8.4": "//third-party:yoke-0.8.4",
    "yoke-derive-0.8.4": "//third-party:yoke-derive-0.8.4",
    "zerofrom-0.1.8": "//third-party:zerofrom-0.1.8",
    "zerofrom-derive-0.1.8": "//third-party:zerofrom-derive-0.1.8",
    "zeroize-1.9.1": "//third-party:zeroize-1.9.1",
    "zerotrie-0.2.5": "//third-party:zerotrie-0.2.5",
    "zerovec-0.11.9": "//third-party:zerovec-0.11.9",
    "zerovec-derive-0.11.7": "//third-party:zerovec-derive-0.11.7",
    "zmij-1.0.23": "//third-party:zmij-1.0.23",
}

CHECKSUMS = {
    "anstream-1.0.0": "824a212faf96e9acacdbd09febd34438f8f711fb84e09a8916013cd7815ca28d",
    "anstyle-1.0.14": "940b3a0ca603d1eade50a4846a2afffd5ef57a9feac2c0e2ec2e14f9ead76000",
    "anstyle-parse-1.0.0": "52ce7f38b242319f7cabaa6813055467063ecdc9d355bbb4ce0c68908cd8130e",
    "anstyle-query-1.1.5": "40c48f72fd53cd289104fc64099abca73db4166ad86ea0b4341abe65af83dadc",
    "anstyle-wincon-3.0.11": "291e6a250ff86cd4a820112fb8898808a366d8f9f58ce16d1f538353ad55747d",
    "anyhow-1.0.103": "2a4385e2e34eb35d6b3efe798b9eb88096925d87726c0798709bf56d9ed84af3",
    "base64-0.22.1": "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6",
    "base64-0.23.1": "ac07cdecf99051d9a5238b80f35af32cdeba5b336e55d957b318b50137e18da5",
    "bitflags-2.13.2": "3ded4057c258ba199e2d26386d3af3780957ecaee6c4ef4041c6b4b8b97c0b06",
    "block-buffer-0.10.4": "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71",
    "bumpalo-3.20.3": "72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649",
    "bytes-1.12.1": "fc652a48c352aef3ea3aed32080501cf3ef6ed5da78602a020c991775b0aff04",
    "cc-1.11.0": "375989decec1a0fa424442bed520fa12ede43a48475ce8c7417d2c69b32b5f6b",
    "cfg-if-1.0.5": "4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600",
    "clap-4.6.7": "aa8876b300ab35ba921adea3dfd70157a46249b33f95c9084ae5709785478946",
    "clap_builder-4.6.7": "ec0797fb7aeb1406c84efac526901f7ec3ead2124f946b494e72879d4b54704d",
    "clap_derive-4.6.7": "f9c751b79415d4e559e3d1fcf128e09e720eb673a06d26cf6f392d37d75b66e0",
    "clap_lex-1.1.1": "1c133bc6a41be0d194c306b5506d15e6feeea7b1d6604bd3f8310dfb2ca96486",
    "colorchoice-1.0.5": "1d07550c9036bf2ae0c684c4297d503f838287c83c53686d05370d0e139ae570",
    "cookie-0.18.2": "1a373e3602691c3cdea496d2f0ee5935151e6168fe87739483c463db1b2f2f87",
    "cookie_store-0.22.1": "15b2c103cf610ec6cae3da84a766285b42fd16aad564758459e6ecf128c75206",
    "cpufeatures-0.2.17": "59ed5838eebb26a2bb2e58f6d5b5316989ae9d08bab10e0e6d103e656d1b0280",
    "crypto-common-0.1.7": "78c8292055d1c1df0cce5d180393dc8cce0abec0a7102adb6c7b1eef6016d60a",
    "deranged-0.5.9": "9e9de72ce2ad1f90dc62fa25f0f430ef85eb4b0d8fa0be4f30373bc40a21d28e",
    "digest-0.10.7": "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292",
    "displaydoc-0.2.7": "c6232dd377dcc64799954cbd3a9bb882e9cdc1308ccd87b1c098f1fb2eaf82a8",
    "document-features-0.2.12": "d4b8a88685455ed29a21542a33abd9cb6510b6b129abadabdcef0f4c55bc8f61",
    "equivalent-1.0.3": "00d174d5400e5e8fd687ad1049e2f578285fa914201b1af7e8b112a4546bd826",
    "errno-0.3.14": "39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb",
    "fastrand-2.5.0": "da7c62ceae207dd37ea5b845da6a0696c799f85e97da1ab5b7910be3c1c80223",
    "find-msvc-tools-0.1.15": "1c66915cc5391097de7a2b0b972e8505aa33c0df401fd3a6811dbdb32881ba48",
    "form_urlencoded-1.2.2": "cb4cb245038516f5f85277875cdaa4f7d2c9a0fa0468de06ed190163b1581fcf",
    "generic-array-0.14.7": "85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a",
    "getrandom-0.2.17": "ff2abc00be7fca6ebc474524697ae276ad847ad0a6b3faa4bcb027e9a4614ad0",
    "getrandom-0.4.3": "300e883d756b2e4ec94e02791f39b04b522276138852cfc41d9fb7e904106099",
    "hashbrown-0.17.1": "ed5909b6e89a2db4456e54cd5f673791d7eca6732202bbf2a9cc504fe2f9b84a",
    "heck-0.5.0": "2304e00983f87ffb38b55b444b5e3b60a884b5d30c0fca7d82fe33449bbe55ea",
    "http-1.5.0": "918d3568bebf352712bc2ef3d46a8bcf1a75b373be6539de198e9105cbbf9ce0",
    "httparse-1.10.1": "6dbf3de79e51f3d586ab4cb9d5c3e2c14aa28ed23d180cf89b4df0454a69cc87",
    "icu_collections-2.4.0": "fe090a42a0d3140fed8c63263ed1fecc02422cb2f4bfe55cd8de2de1b8e105dd",
    "icu_locale_core-2.4.0": "25d29b57c17c6528d0dc4c2141cfa2d08200465acb537a3909c8b9b8a4a984c0",
    "icu_normalizer-2.4.0": "404a3b8c486f205c87fe7f23ab6894d84a4ad76d6e4671ae1981ba7def87c599",
    "icu_normalizer_data-2.4.0": "20f0088bfb7441f7a37fa0c9e20cb1e9cba1cac6b9ce8cd2a6dc44294f8e8cbc",
    "icu_properties-2.4.0": "2ea83d991c56158f7cf898bf86436c1dfe7c07ee82fe88951f2695c9f37a379f",
    "icu_properties_data-2.4.0": "b98f29af82d02b163fe50161388350e743bb3107cc8e8c2f7314cb505faddfcf",
    "icu_provider-2.4.0": "413099315636c588ee8f011190b329b971a09a8fae9d157e01dcae8379468514",
    "idna-1.1.0": "3b0875f23caa03898994f6ddc501886a45c7d3d62d04d2d90788d47be1b1e4de",
    "idna_adapter-1.2.2": "cb68373c0d6620ef8105e855e7745e18b0d00d3bdb07fb532e434244cdb9a714",
    "indexmap-2.14.2": "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855",
    "is_terminal_polyfill-1.70.2": "a6cb138bb79a146c1bd460005623e142ef0181e3d0219cb493e02f7d08a35695",
    "itoa-1.0.18": "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682",
    "leb128fmt-0.1.0": "09edd9e8b54e49e587e4f6295a7d29c3ea94d469cb40ab8ca70b288248a81db2",
    "libc-0.2.191": "65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a",
    "linux-raw-sys-0.12.1": "32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53",
    "litemap-0.8.3": "47d9d19d1d6efa0109d2f65ff4c85cddd50bd572e5a00127ab10987290bcefae",
    "litrs-1.0.1": "4744e383959f0db86ede514b809b1c53251889093803c05267acc7d4e7030d70",
    "log-0.4.35": "5d15f86d12da44cd8d4ec3c73984537eb7f4cadcb9776a575a0a8a05ca11005c",
    "memchr-2.8.3": "cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98",
    "num-conv-0.2.2": "521739c6d2bac4aa25192232afe6841231376b2b26d4d9fae5ecf8ca5772e441",
    "once_cell-1.21.4": "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50",
    "once_cell_polyfill-1.70.2": "384b8ab6d37215f3c5301a95a4accb5d64aa607f1fcb26a11b5303878451b4fe",
    "percent-encoding-2.3.2": "9b4f627cb1b25917193a259e49bdad08f671f8d9708acfd5fe0a8c1455d87220",
    "potential_utf-0.1.6": "d83eb9bc6d8e5cf568e7a1101d60ee05e81ed50ea106026f3d18deeb046d7661",
    "powerfmt-0.2.1": "4a6394b9e965e73d0a289ee54f589087e2c676aedf60885baf52c76b771e4958",
    "proc-macro2-1.0.107": "985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9",
    "quote-1.0.47": "1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001",
    "r-efi-6.0.0": "f8dcc9c7d52a811697d2151c701e0d08956f92b0e24136cf4cf27b57a6a0d9bf",
    "ring-0.17.14": "a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7",
    "rustix-1.1.6": "e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa",
    "rustls-0.23.46": "48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf",
    "rustls-pki-types-1.15.1": "2f4925028c7eb5d1fcdaf196971378ed9d2c1c4efc7dc5d011256f76c99c0a96",
    "rustls-webpki-0.103.16": "4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281",
    "semver-1.0.28": "8a7852d02fc848982e0c167ef163aaff9cd91dc640ba85e263cb1ce46fae51cd",
    "serde-1.0.228": "9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e",
    "serde_core-1.0.228": "41d385c7d4ca58e59fc732af25c3983b67ac852c1a25000afe1175de458b67ad",
    "serde_derive-1.0.228": "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79",
    "serde_json-1.0.150": "e8014e44b4736ed0538adeecded0fce2a272f22dc9578a7eb6b2d9993c74cfb9",
    "serde_spanned-0.6.9": "bf41e0cfaf7226dca15e8197172c295a782857fcb97fad1808a166870dee75a3",
    "sha2-0.10.9": "a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283",
    "shlex-2.0.1": "f8fadd59c855ef2080decdef8ff161eb6661b86933c9d82e5ba29dc602a55aba",
    "smallvec-1.16.4": "d0d5d4b1981b8bbbb7aed204cae5726d683d5d6ac7e74ec54122ea28f315e7c8",
    "stable_deref_trait-1.2.1": "6ce2be8dc25455e1f91df71bfa12ad37d7af1092ae736f3a6cd0e37bc7810596",
    "strsim-0.11.1": "7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f",
    "subtle-2.6.1": "13c2bddecc57b384dee18652358fb23172facb8a2c51ccc10d74c157bdea3292",
    "syn-2.0.119": "872831b642d1a07999a962a351ed35b955ea2cfc8f3862091e2a240a84f17297",
    "syn-3.0.9": "d78c8dee4c7bf0e14673097256fed6142ce9d3b85a408189d07482442145823b",
    "synstructure-0.14.0": "901704edd0dfe137f1987838ee4f259e4e063c31371bdb423f7ae38ec6f77f02",
    "tempfile-3.27.0": "32497e9a4c7b38532efcdebeef879707aa9f794296a4f0244f6f69e9bc8574bd",
    "time-0.3.55": "cdb87b95ec50ddfa440816d227a17b2ccbdda963a316a727fda0fc4334f7d134",
    "time-core-0.1.9": "9e1c906769ad99c88eaa54e728060edef082f8e358ff32030cb7c7d315e81109",
    "time-macros-0.2.32": "7e689342a48d2ea927c87ea50cabf8594854bf940e9310208848d680d668ed85",
    "tinystr-0.8.5": "abc840c238154d58f9e86b2792261edfde565a17696e1cda1a449de27979801a",
    "toml-0.8.23": "dc1beb996b9d83529a9e75c17a1686767d148d70663143c7854d8b4a09ced362",
    "toml_datetime-0.6.11": "22cddaf88f4fbc13c51aebbf5f8eceb5c7c5a9da2ac40a13519eb5b0a0e8f11c",
    "toml_edit-0.22.27": "41fe8c660ae4257887cf66394862d21dbca4a6ddd26f04a3560410406a2f819a",
    "toml_write-0.1.2": "5d99f8c9a7727884afe522e9bd5edbfc91a3312b36a77b5fb8926e4c31a41801",
    "typenum-1.20.1": "b6f5e870be6c3b371b77fe0ee0bafb859fa4964b4404c27de1d380043c4dda20",
    "unicode-ident-1.0.27": "a2c754d6c33795a1c324727428e5a7dedb5b06195f9890bdbcba760d3e246563",
    "unicode-width-0.2.3": "ca718471876992059f251e33aafc334346ada08042b7661dd1fe313a9bced557",
    "untrusted-0.9.0": "8ecb6da28b8a351d773b68d5825ac39017e680750f980f3a1a85cd8dd28a47c1",
    "ureq-3.3.0": "dea7109cdcd5864d4eeb1b58a1648dc9bf520360d7af16ec26d0a9354bafcfc0",
    "ureq-proto-0.6.5": "841583fc36089d7d59419568ae157f43a5a03a2e5a81ba87c77d1596ef46e922",
    "url-2.5.8": "ff67a8a4397373c3ef660812acab3268222035010ab8680ec4215f38ba3d0eed",
    "utf8-zero-0.8.1": "b8c0a043c9540bae7c578c88f91dda8bd82e59ae27c21baca69c8b191aaf5a6e",
    "utf8_iter-1.1.0": "af3bd9d3e571c26fcce9d033b87ea5ab061b8d2ffe9b7a5675d810d68f03edf0",
    "utf8parse-0.2.2": "06abde3611657adf66d383f00b093d7faecc7fa57071cce2578660c9f1010821",
    "utf_types-1.0.0": "3a3b425286980a367278453f3b6d28cdcb134b574e5a35824a6ff5eebad50bb6",
    "version_check-0.9.5": "0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a",
    "wasi-0.11.1+wasi-snapshot-preview1": "ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b",
    "wasm-encoder-0.263.0": "0b883a26aea02eaabe2d2c3fba7eb039ef5cdcb5d5201ae5551e73d5e251a81f",
    "wasmparser-0.263.0": "4f5fa0a054ae153606df820eb0a84213e28cda41bbe04ba2628e71b6ba100fcb",
    "wast-263.0.0": "69d90bcf581b501aa1c358bc5dc0faa979cd375796908860ebcb6d1d70b907c7",
    "webpki-roots-1.0.11": "f5f280bbb2b287bc8e7de7e3852945a0ff50d1bbcb5ed42b6c7a568c546df171",
    "windows-link-0.2.1": "f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5",
    "windows-sys-0.52.0": "282be5f36a8ce781fad8c8ae18fa3f9beff57ec1b52cb3de0789201425d9a33d",
    "windows-sys-0.61.2": "ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc",
    "windows-targets-0.52.6": "9b724f72796e036ab90c1021d4780d4d3d648aca59e491e6b98e725b84e99973",
    "windows_aarch64_gnullvm-0.52.6": "32a4622180e7a0ec044bb555404c800bc9fd9ec262ec147edd5989ccd0c02cd3",
    "windows_aarch64_msvc-0.52.6": "09ec2a7bb152e2252b53fa7803150007879548bc709c039df7627cabbd05d469",
    "windows_i686_gnu-0.52.6": "8e9b5ad5ab802e97eb8e295ac6720e509ee4c243f69d781394014ebfe8bbfa0b",
    "windows_i686_gnullvm-0.52.6": "0eee52d38c090b3caa76c563b86c3a4bd71ef1a819287c19d586d7334ae8ed66",
    "windows_i686_msvc-0.52.6": "240948bc05c5e7c6dabba28bf89d89ffce3e303022809e73deaefe4f6ec56c66",
    "windows_x86_64_gnu-0.52.6": "147a5c80aabfbf0c7d901cb5895d1de30ef2907eb21fbbab29ca94c5b08b1a78",
    "windows_x86_64_gnullvm-0.52.6": "24d5b23dc417412679681396f2b49f3de8c1473deb516bd34410872eff51ed0d",
    "windows_x86_64_msvc-0.52.6": "589f6da84c646204747d1270a2a5661ea66ed1cced2631d546fdfb155959f9ec",
    "winnow-0.7.15": "df79d97927682d2fd8adb29682d1140b343be4ac0f08fd68b7765d9c059d3945",
    "writeable-0.6.4": "3ad82d2a33cdc9674dc7465672f271e096168fcdbe0f799d9e6db8c5892679dc",
    "yoke-0.8.4": "417ef3fcf6c300b7dbd25c85531a746931c02ad536f3cf4786ed1e88223340b3",
    "yoke-derive-0.8.4": "ec8ebde2db3681e8c9980cc27822030e68752690ddfa9473e739aeb4dbde6d71",
    "zerofrom-0.1.8": "0ec05a11813ea801ff6d75110ad09cd0824ddba17dfe17128ea0d5f68e6c5272",
    "zerofrom-derive-0.1.8": "f75b4683f6c7f45248d4d64056a24298c6281e0993356d7d1b4a1a962ef10d4a",
    "zeroize-1.9.1": "e13084392c5e4bc371903e2935a5eaeed24905a7511356b883835e18a78f6879",
    "zerotrie-0.2.5": "4ea269c3bd32f0a32c321907a2ae912ba6f4649bb0fc764a15627e99a7095a3f",
    "zerovec-0.11.9": "f58a2799cfe9a1670c7ea8edbd52ef8edb93cb37a7f83bf31ba9e35d9dae6a40",
    "zerovec-derive-0.11.7": "c3743ddbf47738d929cd1ae2e199d40d0162979dfe31aef8ae646d2bf00bf97a",
    "zmij-1.0.23": "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b",
}
