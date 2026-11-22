/* kit/make/fuzz-diff.h: the differential fuzzer's generic driver shell
 * (PLAN step 5), the part of a port's libFuzzer driver that does not
 * depend on the library it compares. kit/make/fuzz-diff.c implements it;
 * the theseus-proved-port package installs both in
 * /usr/share/proved-port/make/, and a port compiles fuzz-diff.c into its
 * driver beside its own decoding, comparisons and known-divergence table:
 *
 *   clang -fsanitize=fuzzer,undefined -I$(PROVED_PORT_HOME)/make \
 *     $(PROVED_PORT_HOME)/make/fuzz-diff.c driver.c known.c -ldl -o driver
 *
 * The shell loads each library with dlopen by absolute path and
 * RTLD_LOCAL, so the driver links against neither; checks that every
 * symbol it resolves lies in that library's own object (not in a library
 * it depends on) and that the two libraries are two objects; reads the run
 * modes from the environment; and writes the report's generic parts. What
 * each library must report as its version (the version voices) is the
 * port's: it calls its loaded functions and refuses with ppd_die.
 *
 * Environment, under the prefix the port gives ppd_init (P):
 *   P_PREFLIGHT  set: ppd_ready exits 0 (load, check, exit)
 *   P_CONTINUE   set: ppd_finding_end counts instead of aborting, and the
 *                process exits 1 at its end if it counted any (a replay
 *                of many inputs reports every divergence)
 *   P_KNOWN_LOG  a file ppd_known appends one line to, the first time a
 *                process meets each name (fork-mode jobs share it: each
 *                line is one write with O_APPEND)
 * A variable counts as set when it is in the environment, empty or not.
 *
 * Every refusal prints "<name>: <message>" on stderr and exits 1, so a
 * script can run the driver with P_PREFLIGHT set before fuzzing and stop
 * on a nonzero exit. */
#ifndef PROVED_PORT_FUZZ_DIFF_H
#define PROVED_PORT_FUZZ_DIFF_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* One library. The port sets label (in messages) and path (absolute);
 * ppd_open sets the rest. */
struct ppd_lib {
  const char *label;
  const char *path;
  void *handle; /* dlopen's handle */
  void *map;    /* the object's struct link_map (dlinfo RTLD_DI_LINKMAP) */
  const char *file; /* the object's file name, as the loader records it */
};

/* Names the driver in its messages (name) and its environment variables
 * (env_prefix, as P above). Call it first; both strings must outlive the
 * process. Refuses a prefix longer than 200 bytes. */
void ppd_init(const char *name, const char *env_prefix);

/* The name ppd_init was given. */
const char *ppd_name(void);

/* Prints "<name>: <message>\n" on stderr and exits 1. */
void ppd_die(const char *fmt, ...) __attribute__((format(printf, 1, 2), noreturn));

/* Loads l->path with RTLD_NOW | RTLD_LOCAL. Refuses a NULL or relative
 * path and a library that does not load. */
void ppd_open(struct ppd_lib *l);

/* The address of `symbol` in l, refused when l has no such symbol or when
 * it resolves to another object than l's own (dlsym also searches the
 * libraries l depends on). The port casts it to the function's type. */
void *ppd_sym(struct ppd_lib *l, const char *symbol);

/* Refuses two libraries that are one object (one file loaded twice, or
 * through a link to it). */
void ppd_distinct(const struct ppd_lib *a, const struct ppd_lib *b);

/* Ends the preflight: flushes stdout, exits 0 under P_PREFLIGHT, and
 * under P_CONTINUE makes ppd_finding_end count. The port calls it after
 * its own checks and its banner. */
void ppd_ready(void);

/* 1 under P_CONTINUE once ppd_ready has run. */
int ppd_keep_going(void);

/* Counts a known divergence class or a skipped call, `what` saying which
 * ("known divergence", "skipped reference call"): the first time a
 * process meets `name` (compared as a string) it prints
 * "<name>: <what> <name>\n" on stderr and appends "<name>\n" to
 * P_KNOWN_LOG. It keeps a copy of each name, so `name` may be a buffer
 * the caller reuses. Past 64 distinct names it prints and logs each
 * call. */
void ppd_known(const char *what, const char *name);

/* Starts a finding's report: "<name>: DIVERGENCE in <stage>[ <what>]\n"
 * on stderr. what may be NULL. */
void ppd_divergence(const char *stage, const char *what);

/* n code units of `width` bits (8, 16 or 32) as a quoted string: '"' and
 * '\\' escaped, printable ASCII as itself, other units as \xhh at 8 bits
 * and \x{h...} at 16 and 32, then " (n bytes)" at 8 bits or
 * " (n units)". Refuses another width. */
void ppd_put_units(FILE *f, const void *units, size_t n, unsigned width);

/* p as base64 (RFC 4648, with padding), then a newline. */
void ppd_put_base64(FILE *f, const uint8_t *p, size_t n);

/* Ends a finding's report: "  reproducer (base64): <input>\n" on stderr,
 * then abort() so libFuzzer saves the input, or under P_CONTINUE counts
 * it and returns 1. */
int ppd_finding_end(const uint8_t *data, size_t size);

#endif
