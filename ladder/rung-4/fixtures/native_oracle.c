/* Native execution of the unmodified oracle, called with the image and each x in
 * order; prints one bit per x on one line. Same oracle.c, same SQLite, same flags.
 *   native_oracle <image-file> <2^A region bytes> <x count> */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "sqlite3.h"
uint8_t rung4_oracle(unsigned char *img, sqlite3_int64 len, uint64_t x);
int main(int argc, char **argv) {
  if (argc != 3) { fprintf(stderr, "usage: native_oracle image xcount\n"); return 2; }
  FILE *f = fopen(argv[1], "rb");
  if (!f) { fprintf(stderr, "FATAL open\n"); return 2; }
  fseek(f, 0, SEEK_END);
  long len = ftell(f);
  rewind(f);
  unsigned char *img = malloc((size_t)len);
  if (!img || fread(img, 1, (size_t)len, f) != (size_t)len) { fprintf(stderr, "FATAL read\n"); return 2; }
  fclose(f);
  uint64_t count = strtoull(argv[2], 0, 0);
  for (uint64_t x = 0; x < count; x++) {
    uint8_t r = rung4_oracle(img, len, x);
    if (r > 1) { fprintf(stderr, "FATAL out of range result\n"); return 2; }
    putchar('0' + r);
  }
  putchar('\n');
  return 0;
}
