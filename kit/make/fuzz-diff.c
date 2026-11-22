/* kit/make/fuzz-diff.c: the differential fuzzer's generic driver shell;
 * kit/make/fuzz-diff.h documents it. From the PCRE2 port's
 * fuzz/diff/driver.c (its loading, preflight, run modes and report), with
 * the library's names, version checks and comparisons left to the port.
 * The one change in behaviour: a symbol must resolve into the library's
 * own object (its link map), where the port's driver checked only that
 * every symbol resolved into one object. */
#define _GNU_SOURCE
#include "fuzz-diff.h"

#include <dlfcn.h>
#include <fcntl.h>
#include <link.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define ENV_MAX 200

static const char *prog = "fuzz-diff";
static char env_preflight[ENV_MAX + 16];
static char env_continue[ENV_MAX + 16];
static char env_known_log[ENV_MAX + 16];
static int keep_going;
static unsigned long divergences;

void ppd_die(const char *fmt, ...) {
  va_list ap;
  fflush(stdout);
  fprintf(stderr, "%s: ", prog);
  va_start(ap, fmt);
  vfprintf(stderr, fmt, ap);
  va_end(ap);
  fputc('\n', stderr);
  exit(1);
}

void ppd_init(const char *name, const char *env_prefix) {
  if (name != NULL && name[0] != '\0') prog = name;
  if (env_prefix == NULL || env_prefix[0] == '\0')
    ppd_die("ppd_init: no environment prefix");
  if (strlen(env_prefix) > ENV_MAX)
    ppd_die("ppd_init: the environment prefix is longer than %d bytes", ENV_MAX);
  snprintf(env_preflight, sizeof env_preflight, "%s_PREFLIGHT", env_prefix);
  snprintf(env_continue, sizeof env_continue, "%s_CONTINUE", env_prefix);
  snprintf(env_known_log, sizeof env_known_log, "%s_KNOWN_LOG", env_prefix);
}

const char *ppd_name(void) { return prog; }

static void need_init(const char *fn) {
  if (env_preflight[0] == '\0') ppd_die("%s before ppd_init", fn);
}

void ppd_open(struct ppd_lib *l) {
  need_init("ppd_open");
  const char *label = l->label != NULL ? l->label : "library";
  if (l->path == NULL || l->path[0] == '\0') ppd_die("%s: no library path", label);
  if (l->path[0] != '/') ppd_die("%s: %s is not an absolute path", label, l->path);
  l->handle = dlopen(l->path, RTLD_NOW | RTLD_LOCAL);
  if (l->handle == NULL) ppd_die("%s: %s", label, dlerror());
  struct link_map *map = NULL;
  if (dlinfo(l->handle, RTLD_DI_LINKMAP, &map) != 0 || map == NULL)
    ppd_die("%s: %s: dlinfo found no link map", label, l->path);
  l->map = map;
  l->file = map->l_name != NULL && map->l_name[0] != '\0' ? map->l_name : l->path;
}

void *ppd_sym(struct ppd_lib *l, const char *symbol) {
  const char *label = l->label != NULL ? l->label : "library";
  if (l->handle == NULL) ppd_die("%s: ppd_sym before ppd_open", label);
  dlerror();
  void *p = dlsym(l->handle, symbol);
  if (p == NULL) ppd_die("%s: no %s", label, symbol);
  Dl_info info;
  struct link_map *map = NULL;
  if (dladdr1(p, &info, (void **)&map, RTLD_DL_LINKMAP) == 0 || info.dli_fbase == NULL ||
      map == NULL)
    ppd_die("%s: dladdr found no object for %s", label, symbol);
  if (map != l->map)
    ppd_die("%s: %s resolves outside the library's object (to %s)", label, symbol,
            info.dli_fname != NULL ? info.dli_fname : "?");
  return p;
}

void ppd_distinct(const struct ppd_lib *a, const struct ppd_lib *b) {
  if (a->handle == NULL || b->handle == NULL) ppd_die("ppd_distinct before ppd_open");
  if (a->handle == b->handle || a->map == b->map)
    ppd_die("both tables resolve to one object (%s, %s)", a->file, b->file);
}

/* At exit under P_CONTINUE: exit 1 if there were findings. exit() may not
 * be called again from its own handler, so _exit, which flushes no
 * stream: flush them first, or what the driver wrote to stdout after
 * ppd_ready (a file or pipe, so fully buffered) would be lost. */
static void count_divergences(void) {
  if (divergences > 0) {
    fprintf(stderr, "%s: %lu divergences\n", prog, divergences);
    fflush(NULL);
    _exit(1);
  }
}

void ppd_ready(void) {
  need_init("ppd_ready");
  fflush(stdout);
  if (getenv(env_preflight) != NULL) exit(0);
  if (getenv(env_continue) != NULL && !keep_going) {
    keep_going = 1;
    atexit(count_divergences);
  }
}

int ppd_keep_going(void) { return keep_going; }

static void log_known(const char *name) {
  const char *log = getenv(env_known_log);
  if (log == NULL) return;
  int fd = open(log, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0644);
  if (fd < 0) {
    fprintf(stderr, "%s: cannot open %s\n", prog, log);
    return;
  }
  /* One write of one line with O_APPEND: the fork-mode jobs share the
   * file. */
  size_t n = strlen(name);
  char *line = malloc(n + 2);
  if (line == NULL) abort();
  memcpy(line, name, n);
  line[n] = '\n';
  if (write(fd, line, n + 1) != (ssize_t)(n + 1))
    fprintf(stderr, "%s: short write to %s\n", prog, log);
  free(line);
  close(fd);
}

void ppd_known(const char *what, const char *name) {
  need_init("ppd_known");
  static const char *seen[64];
  size_t i;
  for (i = 0; i < sizeof seen / sizeof *seen; i++) {
    if (seen[i] == NULL) {
      /* A copy: the caller's buffer may be reused or gone. */
      seen[i] = strdup(name);
      if (seen[i] == NULL) abort();
      break;
    }
    if (strcmp(seen[i], name) == 0) return;
  }
  fprintf(stderr, "%s: %s %s\n", prog, what, name);
  log_known(name);
}

void ppd_divergence(const char *stage, const char *what) {
  fprintf(stderr, "%s: DIVERGENCE in %s%s%s\n", prog, stage, what ? " " : "",
          what ? what : "");
}

void ppd_put_units(FILE *f, const void *units, size_t n, unsigned width) {
  if (width != 8 && width != 16 && width != 32)
    ppd_die("ppd_put_units: width %u is not 8, 16 or 32", width);
  putc('"', f);
  for (size_t i = 0; i < n; i++) {
    uint32_t c = width == 8    ? ((const uint8_t *)units)[i]
                 : width == 16 ? ((const uint16_t *)units)[i]
                               : ((const uint32_t *)units)[i];
    if (c == '"' || c == '\\')
      fprintf(f, "\\%c", (int)c);
    else if (c >= 0x20 && c < 0x7f)
      putc((int)c, f);
    else if (width == 8)
      fprintf(f, "\\x%02x", (unsigned)c);
    else
      fprintf(f, "\\x{%x}", (unsigned)c);
  }
  fprintf(f, width == 8 ? "\" (%zu bytes)" : "\" (%zu units)", n);
}

void ppd_put_base64(FILE *f, const uint8_t *p, size_t n) {
  static const char A[] =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  for (size_t i = 0; i < n; i += 3) {
    uint32_t v = (uint32_t)p[i] << 16;
    if (i + 1 < n) v |= (uint32_t)p[i + 1] << 8;
    if (i + 2 < n) v |= p[i + 2];
    putc(A[(v >> 18) & 63], f);
    putc(A[(v >> 12) & 63], f);
    putc(i + 1 < n ? A[(v >> 6) & 63] : '=', f);
    putc(i + 2 < n ? A[v & 63] : '=', f);
  }
  putc('\n', f);
}

int ppd_finding_end(const uint8_t *data, size_t size) {
  fprintf(stderr, "  reproducer (base64): ");
  ppd_put_base64(stderr, data, size);
  fflush(stderr);
  if (keep_going) {
    divergences++;
    return 1;
  }
  abort();
}
