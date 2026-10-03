# This fork

sail's fork of [0x676e67/btls](https://github.com/0x676e67/btls). The
`sail` branch tracks upstream `main`, and sail pins revisions of it.

Upstream is merged into `sail`, never rebased onto it, and the branch is
never force-pushed: a pinned revision has to stay reachable. The patches
below are not offered upstream.

## Patches

- **Apple deployment target** (`btls-sys`): BoringSSL is built with
  CMake's `CMAKE_OSX_DEPLOYMENT_TARGET` set from the
  `IPHONEOS_DEPLOYMENT_TARGET` / `TVOS_DEPLOYMENT_TARGET` that rustc links
  for. Without it, CMake targets the SDK's version, and linking for an
  older iOS warns on every object or fails (`___chkstk_darwin` below
  iOS 13).
- **`SslRef::set_connect_state` / `set_accept_state`** (`btls`): a caller
  that wraps an `Ssl` in `SslStream::new` and drives the handshake itself
  could not pick the side without the FFI. sail drives its TCP handshakes
  over memory buffers, but still calls `SSL_set_connect_state` /
  `SSL_set_accept_state` through `btls-sys` itself, so nothing in sail
  uses this patch today: it can be dropped, or sail can move to it.
- **REALITY hooks** (`btls-sys`, `reality.patch` on BoringSSL):
  - `SSL_set_client_hello_finalize_cb` hands the encoded first ClientHello,
    the client random and the X25519 private key to a callback that
    returns the session ID, before the hello enters the transcript.
  - `SSL_set_extra_peer_verify_algorithms` accepts signature algorithms
    the client does not advertise (a REALITY server signs with Ed25519,
    which browser hellos do not list).
- **`SSL_set_ech_grease_shape`** (`btls-sys`): fixes the HPKE AEAD and
  payload length of the ECH GREASE extension, which BoringSSL picks by AES
  hardware and at random. Imitating Firefox needs ChaCha20-Poly1305 and a
  240-byte payload.

## When it could go away

Each patch sail uses would have to be in upstream (or BoringSSL) in an
equivalent form. The deployment-target fix is small and general. The REALITY and ECH GREASE hooks are specific to imitating other
clients, and likely stay sail's: as long as sail speaks REALITY, this fork
stays.

## Prebuilt BoringSSL

BoringSSL, patched as this fork patches it, is compiled once per commit
and target by `.github/workflows/boringssl.yml`, so that a project that
builds sail (or btls) from source need not compile it again.

**Where:** the release tagged `bssl-<full commit>` of the commit a
Cargo.lock pins for `btls-sys` (the `#<commit>` of its `source`), with:

- `boringssl-<target>.tar.gz`: `lib/` (`libcrypto.a` and `libssl.a`, or
  `crypto.lib` and `ssl.lib` for MSVC), `include/`, and `BUILDINFO` (the
  commit, the target, the sail revision whose toolchain built it, the
  runner image, rustc and cmake);
- `SHA256SUMS`, and a build attestation for every file
  (`gh attestation verify <file> -R peakpassvpn/btls`).

A release is made once and its files are never replaced. Not every commit
has one: the workflow is dispatched for the commits sail pins.

Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`i686-unknown-linux-musl`, `armv7-unknown-linux-musleabihf`,
`arm-unknown-linux-musleabi`, `aarch64-linux-android`,
`armv7-linux-androideabi`, `x86_64-linux-android`, `i686-linux-android`.

**Using it:** `scripts/prebuilt-boringssl.sh <Cargo.lock> <target>` finds
the commit, downloads and checks the target's file (SHA256SUMS and the
attestation), unpacks it into a cache, and prints the variables that make
btls-sys link it rather than compile BoringSSL:

    eval "$(scripts/prebuilt-boringssl.sh Cargo.lock x86_64-pc-windows-msvc)"
    cargo build --target x86_64-pc-windows-msvc ...

They are the target's own (`BORING_BSSL_PATH_<target>`,
`BORING_BSSL_INCLUDE_PATH_<target>`, `BORING_BSSL_ASSUME_PATCHED_<target>`),
so a build that also compiles for its host is not handed them there.
bindgen still runs over `include/`, so the build needs libclang as before.

The script needs `gh`, signed in, for the attestation; without it, or if a
check fails, it exits with an error and prints nothing, and the cache keeps
nothing of that download. A build that then runs without the variables
compiles BoringSSL from source as before.

**What the consumer's build must match** (each is what sail builds with):

- **Windows (MSVC):** the dynamic CRT, `/MD`, Rust's default. A build
  with `-C target-feature=+crt-static` needs BoringSSL built with `/MT`,
  which is not published. BoringSSL is built with its assembly (NASM).
- **macOS:** deployment target 13.0 or later (`MACOSX_DEPLOYMENT_TARGET`).
- **Linux (gnu):** glibc 2.35 or later; built on Ubuntu 22.04.
- **Linux (musl), Android:** sail's cross toolchains
  (`scripts/install_cross_toolchain.sh` in sail); `i686` with `-msse2`.
- **All:** no LTO across the C libraries, and no `prefix-symbols`
  feature: the libraries' symbols are not prefixed. Position-independent
  code comes from the flags the `cc` crate hands CMake (`-fPIC` where
  the target wants it); btls-sys does not ask for it itself, and the
  workflow does not check it per target.
- **Features:** btls-sys's default features. A feature that applies a
  patch of its own (`rpk`, `underscore-wildcards`,
  `relax-cert-validation`) is not in these libraries.
