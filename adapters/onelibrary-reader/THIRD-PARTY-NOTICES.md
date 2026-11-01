# Third-party notices — isolated OneLibrary reader

The isolated `adapters/onelibrary-reader` workspace builds SQLCipher through `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature. These components are not linked into the main OxiDJ workspace. When redistributing this reader or its binary, include the notices and license texts in `THIRD-PARTY-LICENSES/`:

- **SQLCipher** — Zetetic LLC and contributors. BSD 3-Clause license; full required copyright, conditions, and disclaimer are reproduced in `THIRD-PARTY-LICENSES/SQLCIPHER-BSD-3-CLAUSE.txt` from the locked `libsqlite3-sys` vendored SQLCipher source.
- **OpenSSL** — OpenSSL Project contributors. The applicable MIT and Apache-2.0 license texts are included as `OPENSSL-MIT.txt` and `OPENSSL-APACHE-2.0.txt`; the upstream OpenSSL notice is included as `OPENSSL-UPSTREAM-NOTICE.txt`. These are copied from the locked vendored `openssl-src` source distribution.

OxiDJ does not modify these third-party sources. The root ADR-009 allowlist is unchanged. `cargo deny --locked check licenses` for the isolated reader is mandatory.
