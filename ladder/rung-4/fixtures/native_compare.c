/* §17.7 comparison harness: the materialised module, with its own link-time image, prints the
 * oracle's bit for each x. Linked by the driver against bennett-runtimes, never host libc.
 * -DRUNG4_BITS -DRUNG4_LEN from the gate; the region function is the materialised oracle. */
#include <stdint.h>
int putchar(int);
extern uint8_t rung4_oracle(unsigned char *img, long long len, uint64_t x);
extern unsigned char rung4_db_image[];
int main(void) {
  for (uint64_t x = 0; x < (UINT64_C(1) << RUNG4_BITS); x++) {
    uint8_t r = rung4_oracle(rung4_db_image, RUNG4_LEN, x);
    if (r > 1) __builtin_trap();
    putchar('0' + r);
  }
  putchar('\n');
  return 0;
}
