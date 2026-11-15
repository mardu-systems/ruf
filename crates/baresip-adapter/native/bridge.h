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
struct sip_account_config {
    const char *registrar, *username, *password, *auth_username, *outbound_proxy, *monitored_extensions;
    int transport, media_security, enabled;
};
struct sip_sound_settings {
    const char *input_device, *output_device, *ring_device;
    int ringtone_enabled, ringtone_variant;
    unsigned ringtone_volume;
};
int sip_engine_account(void *handle, int operation, uint32_t account_id,
                       const struct sip_account_config *config);
int sip_engine_command(void *handle, int operation, uint32_t account_id,
                       uint64_t generation, const char *destination, int enabled);
int sip_engine_sound(void *handle, int restore, const struct sip_sound_settings *settings);
int sip_engine_audio(void *handle, int operation, uint64_t generation, const char *device);
int sip_engine_stop(void *handle);
#endif
