/* The application side of SQLITE_OS_OTHER=1 (sqlite.org/compile.html, sqlite.org/vfs.html):
 * SQLite itself is not edited; the build option makes the application supply
 * sqlite3_os_init/sqlite3_os_end and register a default VFS. The main database never
 * comes through this VFS: it comes through SQLite's own memdb VFS (sqlite3_deserialize)
 * over the region; memdb delegates only randomness, clock and sleep here.
 * No real files exist. Randomness and clock are fixed classical constants (North Star
 * §4.2: everything but the superposed input is fixed before entry). */
#include <string.h>
#include "sqlite3.h"

static int nofsOpen(sqlite3_vfs *v, sqlite3_filename n, sqlite3_file *f, int fl, int *o) {
  (void)v; (void)n; (void)fl; (void)o;
  f->pMethods = 0;
  return SQLITE_CANTOPEN;
}
static int nofsDelete(sqlite3_vfs *v, const char *n, int s) {
  (void)v; (void)n; (void)s;
  return SQLITE_IOERR_DELETE;
}
static int nofsAccess(sqlite3_vfs *v, const char *n, int fl, int *r) {
  (void)v; (void)n; (void)fl;
  *r = 0;
  return SQLITE_OK;
}
static int nofsFullPathname(sqlite3_vfs *v, const char *n, int size, char *out) {
  size_t len = strlen(n);
  (void)v;
  if (len >= (size_t)size) return SQLITE_CANTOPEN;
  memcpy(out, n, len + 1);
  return SQLITE_OK;
}
static int nofsRandomness(sqlite3_vfs *v, int n, char *out) {
  (void)v;
  for (int i = 0; i < n; i++) out[i] = (char)(0xA5 ^ (i * 29));
  return n;
}
static int nofsSleep(sqlite3_vfs *v, int us) { (void)v; return us; }
static int nofsCurrentTime(sqlite3_vfs *v, double *t) {
  (void)v;
  *t = 2461342.5;
  return SQLITE_OK;
}
static int nofsCurrentTimeInt64(sqlite3_vfs *v, sqlite3_int64 *t) {
  (void)v;
  *t = (sqlite3_int64)212659992000000LL;
  return SQLITE_OK;
}
static int nofsGetLastError(sqlite3_vfs *v, int n, char *buf) {
  (void)v;
  if (n > 0) buf[0] = 0;
  return 0;
}
static sqlite3_vfs nofs_vfs = {
  2, (int)sizeof(sqlite3_file), 512, 0, "bennett-nofs", 0,
  nofsOpen, nofsDelete, nofsAccess, nofsFullPathname,
  0, 0, 0, 0, /* SQLITE_OMIT_LOAD_EXTENSION */
  nofsRandomness, nofsSleep, nofsCurrentTime, nofsGetLastError,
  nofsCurrentTimeInt64, 0, 0, 0
};
int sqlite3_os_init(void) { return sqlite3_vfs_register(&nofs_vfs, 1); }
int sqlite3_os_end(void) { return SQLITE_OK; }
