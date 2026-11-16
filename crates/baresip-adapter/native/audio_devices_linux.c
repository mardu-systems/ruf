/* PulseAudio metadata only; never creates a recording/playback stream. */
#define _POSIX_C_SOURCE 200809L
#include <pulse/pulseaudio.h>
#include <errno.h>
#include <string.h>
#include <time.h>
#include "audio_devices.h"

struct inventory {
    sip_audio_device_h *callback;
    void *arg;
    int error;
};

static void source_info(pa_context *context, const pa_source_info *info, int end, void *arg) {
    struct inventory *inventory = arg;
    (void)context;
    if (end < 0) inventory->error = ENODEV;
    /* Monitor sources are valid endpoints, but are not microphones. */
    if (end || !info || !info->name || info->monitor_of_sink != PA_INVALID_INDEX) return;
    if (strlen(info->name) <= 127)
        inventory->callback(1, info->name, info->description ? info->description : info->name, 1, inventory->arg);
}

static void sink_info(pa_context *context, const pa_sink_info *info, int end, void *arg) {
    struct inventory *inventory = arg;
    (void)context;
    if (end < 0) inventory->error = ENODEV;
    if (end || !info || !info->name) return;
    if (strlen(info->name) <= 127)
        inventory->callback(0, info->name, info->description ? info->description : info->name, 1, inventory->arg);
}

static uint64_t milliseconds(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now)) return 0;
    return (uint64_t)now.tv_sec * 1000 + (uint64_t)now.tv_nsec / 1000000;
}

static int iterate(pa_mainloop *loop, uint64_t deadline) {
    if (milliseconds() >= deadline) return ETIMEDOUT;
    if (pa_mainloop_iterate(loop, 0, NULL) < 0) return ENODEV;
    const struct timespec delay = {0, 1000000};
    nanosleep(&delay, NULL);
    return 0;
}

int sip_linux_audio_devices(sip_audio_device_h *callback, void *arg) {
    pa_mainloop *loop = pa_mainloop_new();
    if (!loop) return ENOMEM;
    pa_context *context = pa_context_new(pa_mainloop_get_api(loop), "Ruf device inventory");
    if (!context) { pa_mainloop_free(loop); return ENOMEM; }
    int error = pa_context_connect(context, NULL, PA_CONTEXT_NOAUTOSPAWN, NULL) < 0 ? ENODEV : 0;
    uint64_t deadline = milliseconds() + 3000;
    while (!error && pa_context_get_state(context) != PA_CONTEXT_READY) {
        if (!PA_CONTEXT_IS_GOOD(pa_context_get_state(context))) error = ENODEV;
        else error = iterate(loop, deadline);
    }
    struct inventory inventory = {callback, arg, 0};
    for (int input = 0; !error && input <= 1; ++input) {
        pa_operation *operation = input
            ? pa_context_get_source_info_list(context, source_info, &inventory)
            : pa_context_get_sink_info_list(context, sink_info, &inventory);
        if (!operation) { error = ENODEV; break; }
        while (!error && pa_operation_get_state(operation) == PA_OPERATION_RUNNING) {
            if (!PA_CONTEXT_IS_GOOD(pa_context_get_state(context))) error = ENODEV;
            else error = iterate(loop, deadline);
        }
        if (!error && pa_operation_get_state(operation) != PA_OPERATION_DONE) error = ENODEV;
        if (pa_operation_get_state(operation) == PA_OPERATION_RUNNING) pa_operation_cancel(operation);
        pa_operation_unref(operation);
        if (!error) error = inventory.error;
    }
    pa_context_disconnect(context);
    pa_context_unref(context);
    pa_mainloop_free(loop);
    return error;
}
