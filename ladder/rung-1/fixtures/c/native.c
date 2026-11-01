#include <stdint.h>
#include <stdio.h>
extern int8_t r1_sat_add(int8_t, int8_t), r1_gcd(int8_t, int8_t);
extern int8_t r1_popcount(int8_t), r1_sign_abs(int8_t), r1_lookup(int8_t);
int main(void) {
    for (unsigned k = 0; k < 5; ++k)
        for (unsigned a = 0; a < 256; ++a)
            for (unsigned b = 0; b < (k < 2 ? 256 : 1); ++b) {
                int8_t x = (int8_t)a, y = (int8_t)b;
                int8_t v = k == 0 ? r1_sat_add(x,y) : k == 1 ? r1_gcd(x,y) :
                           k == 2 ? r1_popcount(x) : k == 3 ? r1_sign_abs(x) : r1_lookup(x);
                if (putchar((uint8_t)v) == EOF) return 1;
            }
    return 0;
}
