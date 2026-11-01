/* The quantum programmer's translation unit (dialect §16.7, §19.3: the normative route):
 * Grover over the unmodified SQLite oracle. It never touches SQLite's source; it names the
 * region function, binds the database image to MEM_IN, its length to ARG and the rowid register
 * to IN. Parameters come from the gate as -D (no value is computed from a measured outcome):
 *   RUNG4_BITS (n, N = 2^n rows), RUNG4_A (region 2^A bytes), RUNG4_LEN (image bytes),
 *   RUNG4_ITERS (k), RUNG4_B_STEPS, RUNG4_B_HEAP, RUNG4_B_STACK, RUNG4_B_DEPTH.
 * The database is fixed classically before entry: rung4_db_image is the file image built by
 * host_build on the same SQLite, as a link-time global, zero-padded to 2^A bytes
 * (spike route A'), XORed into a clean register by __bennett_qprep. */
#include <stdint.h>
#include "bennett_markers.h"

extern uint8_t rung4_oracle(unsigned char *img, long long len, uint64_t x);
extern const uint8_t rung4_db_image[];

#define WIRES (UINT64_C(8) << RUNG4_A)
#define BYTES (UINT64_C(1) << RUNG4_A)

static void hadamards(bennett_q r) {
  for (uint64_t i = 0; i < RUNG4_BITS; i++) __bennett_gate(BENNETT_GATE_H, r, i);
}
static void paulis(bennett_q r) {
  for (uint64_t i = 0; i < RUNG4_BITS; i++) __bennett_gate(BENNETT_GATE_X, r, i);
}
static void phase_oracle(bennett_q db, bennett_q row, bennett_q t, bennett_q f) {
  BENNETT_ORACLE_DESC_P(3) d;
  bennett_oracle_desc_init(&d.h, 3, 0, t, f);
  bennett_bind_set(&d.param[0], BENNETT_BIND_MEM_IN, 0, db, 0, BYTES);
  bennett_bind_set(&d.param[1], BENNETT_BIND_ARG, 0, 0, RUNG4_LEN, 0);
  bennett_bind_set(&d.param[2], BENNETT_BIND_IN, BENNETT_MODE_ZEXT, row, 0, 0);
  bennett_budget b = { RUNG4_B_STEPS, RUNG4_B_HEAP, RUNG4_B_STACK, RUNG4_B_DEPTH, 0 };
  __bennett_oracle((const void *)rung4_oracle, &b, &d.h);
}
/* Multi-controlled Z on the rowid register by a clean-ancilla CCX ladder. */
static void diffusion(bennett_q row, bennett_q anc) {
  const uint64_t top = RUNG4_BITS - 1;
  hadamards(row);
  paulis(row);
  __bennett_gate(BENNETT_GATE_H, row, top);
  if (top == 1) {
    __bennett_gate2(BENNETT_GATE2_CX, row, 0, row, top);
  } else if (top == 2) {
    __bennett_gate3(BENNETT_GATE3_CCX, row, 0, row, 1, row, top);
  } else {
    __bennett_gate3(BENNETT_GATE3_CCX, row, 0, row, 1, anc, 0);
    for (uint64_t i = 2; i < top - 1; i++)
      __bennett_gate3(BENNETT_GATE3_CCX, row, i, anc, i - 2, anc, i - 1);
    __bennett_gate3(BENNETT_GATE3_CCX, row, top - 1, anc, top - 3, row, top);
    for (uint64_t i = top - 2; i >= 2; i--)
      __bennett_gate3(BENNETT_GATE3_CCX, row, i, anc, i - 2, anc, i - 1);
    __bennett_gate3(BENNETT_GATE3_CCX, row, 0, row, 1, anc, 0);
  }
  __bennett_gate(BENNETT_GATE_H, row, top);
  paulis(row);
  hadamards(row);
}
/* One element (x, t, e) of the §5.1 part 1 domain on the whole final register tuple: prepares the
 * basis element, applies the oracle once, reads every interface wire by measurement of the
 * deterministic state, returns x' | t' << n | e' << (n + 1), then proves the database register
 * restored: XORing the image back out leaves it clean, and qfree of a dirty register is a fault. */
uint64_t rung4_tuple(uint64_t x, uint64_t tv, uint64_t ev) {
  bennett_q db = __bennett_qalloc(BENNETT_CLASS_ANY, WIRES);
  bennett_q row = __bennett_qalloc(BENNETT_CLASS_ANY, RUNG4_BITS);
  bennett_q t = __bennett_qalloc(BENNETT_CLASS_ANY, 1);
  bennett_q f = __bennett_qalloc(BENNETT_CLASS_ANY, 4);
  __bennett_qprep(db, rung4_db_image, BYTES);
  for (uint64_t i = 0; i < RUNG4_BITS; i++) if ((x >> i) & 1) __bennett_gate(BENNETT_GATE_X, row, i);
  if (tv & 1) __bennett_gate(BENNETT_GATE_X, t, 0);
  for (uint64_t i = 0; i < 4; i++) if ((ev >> i) & 1) __bennett_gate(BENNETT_GATE_X, f, i);
  phase_oracle(db, row, t, f);
  uint64_t r = 0;
  for (uint64_t i = 0; i < RUNG4_BITS; i++) r |= (uint64_t)__bennett_measure(row, i) << i;
  uint64_t tout = __bennett_measure(t, 0);
  r |= tout << RUNG4_BITS;
  for (uint64_t i = 0; i < 4; i++) r |= (uint64_t)__bennett_measure(f, i) << (RUNG4_BITS + 1 + i);
  for (uint64_t i = 0; i < RUNG4_BITS; i++) if ((r >> i) & 1) __bennett_gate(BENNETT_GATE_X, row, i);
  if (tout) __bennett_gate(BENNETT_GATE_X, t, 0);
  for (uint64_t i = 0; i < 4; i++) if ((r >> (RUNG4_BITS + 1 + i)) & 1) __bennett_gate(BENNETT_GATE_X, f, i);
  __bennett_qprep(db, rung4_db_image, BYTES);
  __bennett_qfree(f);
  __bennett_qfree(t);
  __bennett_qfree(row);
  __bennett_qfree(db);
  return r;
}
/* Returns the measured rowid register x; the matched row is rowid x + 1. */
uint64_t rung4_grover(void) {
  bennett_q db = __bennett_qalloc(BENNETT_CLASS_ANY, WIRES);
  bennett_q row = __bennett_qalloc(BENNETT_CLASS_ANY, RUNG4_BITS);
  bennett_q t = __bennett_qalloc(BENNETT_CLASS_ANY, 1);
  bennett_q f = __bennett_qalloc(BENNETT_CLASS_ANY, 4);
  bennett_q anc = __bennett_qalloc(BENNETT_CLASS_ANY, RUNG4_BITS);
  __bennett_qprep(db, rung4_db_image, BYTES);
  hadamards(row);
  __bennett_gate(BENNETT_GATE_X, t, 0);
  __bennett_gate(BENNETT_GATE_H, t, 0);
  for (uint64_t k = 0; k < RUNG4_ITERS; k++) {
    phase_oracle(db, row, t, f);
    diffusion(row, anc);
  }
  uint64_t r = 0;
  for (uint64_t i = 0; i < RUNG4_BITS; i++) r |= (uint64_t)__bennett_measure(row, i) << i;
  __bennett_gate(BENNETT_GATE_H, t, 0);
  __bennett_gate(BENNETT_GATE_X, t, 0);
  __bennett_qprep(db, rung4_db_image, BYTES);
  for (uint64_t i = 0; i < RUNG4_BITS; i++) if ((r >> i) & 1) __bennett_gate(BENNETT_GATE_X, row, i);
  /* f stays clean at fault rate zero; a nonzero fault register is not freed clean and is loud. */
  __bennett_qfree(anc);
  __bennett_qfree(f);
  __bennett_qfree(t);
  __bennett_qfree(row);
  __bennett_qfree(db);
  return r;
}
