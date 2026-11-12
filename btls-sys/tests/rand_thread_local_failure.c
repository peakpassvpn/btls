// Force BoringSSL's RAND thread-local setup to fail. RAND_bytes must use
// its stack fallback without freeing the thread state a second time.
// GNU ld wraps calls from the static library only, leaving sanitizer/runtime
// thread-local storage alone. On macOS, two-level namespace binding keeps the
// executable override separate from the runtime shared libraries.
#include <openssl/rand.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>

static unsigned setup_failures;

#ifdef __APPLE__
int pthread_setspecific(pthread_key_t key, const void *value) {
#else
int __wrap_pthread_setspecific(pthread_key_t key, const void *value) {
#endif
  (void)key;
  (void)value;
  ++setup_failures;
  return EAGAIN;
}

int main(void) {
  unsigned char out[32];
  for (unsigned i = 0; i < 2; ++i) {
    if (RAND_bytes(out, sizeof(out)) != 1) {
      fputs("RAND_bytes failed instead of using its stack fallback\n", stderr);
      return 1;
    }
  }
  if (setup_failures < 2) {
    fputs("thread-local setup failure was not exercised\n", stderr);
    return 1;
  }
  puts("RAND_bytes survives repeated thread-local setup failures");
  return 0;
}
