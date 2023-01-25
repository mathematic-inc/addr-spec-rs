#include "ascii.h"

#include <string.h>

size_t ascii_escape(char esc_chr, const char *src, size_t len, char *dst,
                    const char *cntl_chrs, size_t cntl_len) {
  size_t size = 0;
  const char *end = src + len;
  for (; src < end; ++src) {
    char chr = *src;
    if (chr == esc_chr || memchr(cntl_chrs, chr, cntl_len)) {
      *dst++ = esc_chr;
      ++size;
    }
    *dst++ = chr;
    ++size;
  }
  return size;
}