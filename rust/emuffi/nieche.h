
#ifndef NIECHE_H
#define NIECHE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NiecheSession NiecheSession;

uint32_t nieche_abi_version(void);

NiecheSession *nieche_open(const char *path);

void           nieche_close(NiecheSession *s);

int32_t nieche_boot(NiecheSession *s);

void    nieche_stop(NiecheSession *s);

void nieche_size(NiecheSession *s, uint32_t *w, uint32_t *h);

size_t   nieche_step(NiecheSession *s, uint8_t *out, size_t cap);
uint64_t nieche_frame_no(NiecheSession *s);

void nieche_set_keys(NiecheSession *s, uint32_t mask);

void nieche_set_touch(NiecheSession *s, int32_t x, int32_t y, int32_t state);

void nieche_soft_key(NiecheSession *s, int32_t side);

uint32_t nieche_nonblank(NiecheSession *s);
uint32_t nieche_screens(NiecheSession *s);

size_t   nieche_name(NiecheSession *s, uint8_t *out, size_t cap);

size_t nieche_take_events(NiecheSession *s, uint8_t *out, size_t cap);

size_t nieche_take_logs(NiecheSession *s, uint8_t *out, size_t cap);

#ifdef __cplusplus
}
#endif
#endif
