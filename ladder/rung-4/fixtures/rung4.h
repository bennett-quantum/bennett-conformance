/* Rung 4 corpus: the query text and the row generator contract.
 * Row generator and predicate match the feasibility spike's program
 * (bennett-dialect-spec/docs/spikes/sqlite/REPORT.md §1.1) so its budget numbers
 * remain comparable; independently restated in check_gate.py/gate.py (Python). */
#ifndef RUNG4_H
#define RUNG4_H
#define RUNG4_SCHEMA \
  "CREATE TABLE people(id INTEGER PRIMARY KEY, name TEXT NOT NULL," \
  " city TEXT NOT NULL, age INTEGER NOT NULL, score INTEGER NOT NULL)"
#define RUNG4_QUERY \
  "SELECT 1 FROM people WHERE rowid = ?1" \
  " AND name LIKE '%an%'" \
  " AND upper(substr(city, 1, 1)) IN ('B', 'L', 'M')" \
  " AND age BETWEEN 25 AND 60" \
  " AND (score * 7 + age) % 11 = 3"
#endif
