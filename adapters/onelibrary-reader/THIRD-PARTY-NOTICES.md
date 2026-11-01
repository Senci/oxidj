# Third-party notices — isolated OneLibrary reader

The isolated `adapters/onelibrary-reader` workspace builds SQLCipher through `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature. These components are not linked into the main OxiDJ workspace.

- **SQLCipher** — Zetetic LLC and contributors. Distributed under the BSD 3-Clause license. The corresponding license text is provided by the vendored SQLCipher source used by `libsqlite3-sys` in the locked isolated dependency graph.
- **OpenSSL** — OpenSSL Project contributors. The vendored OpenSSL source is included by the locked `openssl-src` dependency and retains its upstream license and notice files.

OxiDJ does not modify these third-party sources. Consult the package license/notice files in the Cargo registry or the corresponding vendored source when redistributing the isolated reader. The root ADR-009 allowlist is unchanged; the isolated locked `cargo deny check licenses` gate is mandatory.
