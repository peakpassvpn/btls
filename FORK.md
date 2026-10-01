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
