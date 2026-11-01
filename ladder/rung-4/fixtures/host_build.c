/* Classical, before entry: builds the database with ordinary INSERTs on the same
 * unmodified SQLite and serialises it to its file image.
 *   host_build <N> <image-out>     (prints: "image_bytes=<k>")
 * The rows come from splitmix64, restated independently in gate.py. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "sqlite3.h"
#include "rung4.h"

static const char *const FIRST[16] = {
  "Ada", "Brian", "Carmen", "Dmitri", "Elena", "Frank", "Grace", "Hans",
  "Ingrid", "Juan", "Kenji", "Lena", "Morgan", "Nadia", "Oscar", "Priya"};
static const char *const LAST[8] = {
  "Anders", "Baker", "Chandra", "Diaz", "Evans", "Fujita", "Gallo", "Hansen"};
static const char *const CITY[16] = {
  "Berlin", "Lagos", "Madrid", "Boston", "Lima", "Mumbai", "Oslo", "Paris",
  "Quito", "Rome", "Seoul", "Tokyo", "Utrecht", "Vienna", "Warsaw", "Zagreb"};

static uint64_t splitmix64(uint64_t x) {
  x += 0x9e3779b97f4a7c15ULL;
  x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
  x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
  return x ^ (x >> 31);
}
static void check(sqlite3 *db, int rc, int want, const char *what) {
  if (rc != want) {
    fprintf(stderr, "FATAL %s: rc=%d %s\n", what, rc, db ? sqlite3_errmsg(db) : "");
    exit(2);
  }
}
int main(int argc, char **argv) {
  if (argc != 3) { fprintf(stderr, "usage: host_build N image\n"); return 2; }
  uint64_t n = strtoull(argv[1], 0, 0);
  sqlite3 *db;
  sqlite3_stmt *ins;
  check(0, sqlite3_open(":memory:", &db), SQLITE_OK, "open");
  check(db, sqlite3_exec(db, RUNG4_SCHEMA, 0, 0, 0), SQLITE_OK, "schema");
  check(db, sqlite3_exec(db, "BEGIN", 0, 0, 0), SQLITE_OK, "begin");
  check(db, sqlite3_prepare_v2(db, "INSERT INTO people VALUES(?1,?2,?3,?4,?5)", -1, &ins, 0), SQLITE_OK, "prepare");
  for (uint64_t rowid = 1; rowid <= n; rowid++) {
    uint64_t h = splitmix64(rowid * 0x2545F4914F6CDD1DULL + 7);
    char name[24], city[12];
    snprintf(name, sizeof name, "%s %s", FIRST[h & 15], LAST[(h >> 4) & 7]);
    snprintf(city, sizeof city, "%s", CITY[(h >> 7) & 15]);
    sqlite3_bind_int64(ins, 1, (sqlite3_int64)rowid);
    sqlite3_bind_text(ins, 2, name, -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(ins, 3, city, -1, SQLITE_TRANSIENT);
    sqlite3_bind_int64(ins, 4, 18 + (sqlite3_int64)((h >> 11) % 73));
    sqlite3_bind_int64(ins, 5, (sqlite3_int64)((h >> 20) % 1000));
    check(db, sqlite3_step(ins), SQLITE_DONE, "insert");
    check(db, sqlite3_reset(ins), SQLITE_OK, "reset");
  }
  check(db, sqlite3_finalize(ins), SQLITE_OK, "finalize");
  check(db, sqlite3_exec(db, "COMMIT", 0, 0, 0), SQLITE_OK, "commit");
  sqlite3_int64 len = 0;
  unsigned char *image = sqlite3_serialize(db, "main", &len, 0);
  if (!image) { fprintf(stderr, "FATAL serialize\n"); return 2; }
  FILE *f = fopen(argv[2], "wb");
  if (!f || fwrite(image, 1, (size_t)len, f) != (size_t)len || fclose(f)) {
    fprintf(stderr, "FATAL write %s\n", argv[2]);
    return 2;
  }
  printf("image_bytes=%lld\n", (long long)len);
  return 0;
}
