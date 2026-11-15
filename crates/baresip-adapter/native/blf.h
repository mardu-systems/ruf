/* Account-owned BLF subscriptions; all operations run on the native event loop. */
#ifndef SIP_BLF_H
#define SIP_BLF_H

#include <stdint.h>

struct ua;
struct sip_blf;

/* body == NULL invalidates observed state. Nonzero requests a full resync. */
typedef int (sip_blf_handler)(uint32_t account_id, const char *extension,
                             const char *body, void *arg);

/* Empty or at most 32 distinct 1–32 digit extensions separated by newlines. */
int sip_blf_validate_targets(const char *targets);
/* Allocate configuration only. Registration evidence starts subscriptions. */
int sip_blf_alloc(struct sip_blf **blfp, struct ua *ua, uint32_t account_id,
                  const char *registrar, int transport, const char *targets,
                  sip_blf_handler *handler, void *arg);
void sip_blf_registered(struct sip_blf *blf);
void sip_blf_suspend(struct sip_blf *blf);

#endif
