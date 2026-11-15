/* Private synchronous ABI. Borrowed inputs are copied before returning;
 * callbacks borrow all strings only until the callback returns. */
#ifndef SIP_BRIDGE_H
#define SIP_BRIDGE_H
#include <stdint.h>
#define SIP_CODE(name, value) name = value,
enum {
#include "protocol.def"
};
#undef SIP_CODE
struct sip_event {
    int kind, code;
    uint32_t account_id;
    uint64_t call_generation;
    const char *caller_name, *caller_number, *detail;
};
typedef void (*event_callback)(const struct sip_event *event, void *arg);
/* Only fields used by operation are populated; all strings are borrowed. */
struct sip_request {
    int operation;
    uint32_t account_id;
    uint64_t call_generation;
    int transport, media_security, enabled, ringtone_variant;
    unsigned ringtone_volume;
    const char *registrar, *username, *password, *auth_username, *outbound_proxy, *monitored_extensions;
    const char *destination, *device, *input_device, *output_device, *ring_device;
};
int sip_engine_request(void *handle, const struct sip_request *request);
#endif
