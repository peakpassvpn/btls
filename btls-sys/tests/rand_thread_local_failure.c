// Force BoringSSL's RAND thread-local setup to fail. RAND_bytes must use
// its stack fallback without freeing the thread state a second time.
// Run as a separate executable: this symbol intentionally overrides pthreads.
#include <openssl/rand.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>

static unsigned setup_failures;

int pthread_setspecific(pthread_key_t key, const void *value) {
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
