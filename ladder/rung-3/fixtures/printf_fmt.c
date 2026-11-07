#include <limits.h>
#include <stdio.h>

int main(void) {
    printf("text=%s signed=%d unsigned=%u hex=%x char=%c\n", "musl", -42, 42u, 0x2au, 'Z');
    printf("width=[%8s][%6d][%6u][%6x][%3c]\n", "xy", -7, 9u, 0xabu, 'Q');
    printf("precision=[%.3s][%.5d][%.5u][%.4x]\n", "abcdef", -12, 23u, 0xabu);
    printf("extremes=%d,%d,%u,%x zero=[%.0d]\n", INT_MIN, INT_MAX, UINT_MAX, UINT_MAX, 0);
    printf("buffered=%s:%d:%u:%x:%c", "tail", -1, 0u, 0x10u, '!');
    return 0;
}
