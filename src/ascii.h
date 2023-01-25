#ifndef SRC_ASCII_H_
#define SRC_ASCII_H_

// NOLINTBEGIN
#include <stddef.h>

size_t ascii_escape(char esc_chr, const char *src, size_t len, char *dst,
                    const char *cntl_chrs, size_t cntl_len);

// NOLINTEND

#endif /* SRC_ASCII_H_ */
