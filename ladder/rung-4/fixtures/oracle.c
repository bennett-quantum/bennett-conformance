/* The rung-4 oracle: a real SQL query over a file image, on stock SQLite.
 * `img` is bound MEM_IN (the quantum-addressed memory of North Star §8 rung 4),
 * `len` is a classical ARG, `x` is the superposed register (IN, zero-extended).
 * The bound rowid is x + 1, so x in [0, 2^n) enumerates N = 2^n rows.
 * Every SQLite error traps: any such basis input is a fault (rung requires rate zero).
 * The result is 1 iff the query returned the row. */
#include <stdint.h>
#include "sqlite3.h"
#include "rung4.h"

uint8_t rung4_oracle(unsigned char *img, sqlite3_int64 len, uint64_t x) {
  sqlite3 *db = 0;
  sqlite3_stmt *st = 0;
  uint8_t out = 0;
  if (sqlite3_open(":memory:", &db) != SQLITE_OK) __builtin_trap();
  if (sqlite3_deserialize(db, "main", img, len, len, SQLITE_DESERIALIZE_READONLY) != SQLITE_OK)
    __builtin_trap();
  if (sqlite3_prepare_v2(db, RUNG4_QUERY, -1, &st, 0) != SQLITE_OK) __builtin_trap();
  if (sqlite3_bind_int64(st, 1, (sqlite3_int64)(x + 1)) != SQLITE_OK) __builtin_trap();
  switch (sqlite3_step(st)) {
    case SQLITE_ROW: out = 1; break;
    case SQLITE_DONE: out = 0; break;
    default: __builtin_trap();
  }
  if (sqlite3_finalize(st) != SQLITE_OK) __builtin_trap();
  if (sqlite3_close(db) != SQLITE_OK) __builtin_trap();
  return out;
}
