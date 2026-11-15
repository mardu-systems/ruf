/* Compile the shipped WASAPI worker sources against fault-injected COM calls.
 * This verifies lifecycle logic, not Windows ABI, drivers or physical audio. */
#undef NDEBUG
#include <assert.h>
#include <stdatomic.h>
#include <stdio.h>
#include <re.h>
#include <rem.h>
#include <baresip.h>
#include <re_atomic.h>
#include "wasapi-stubs/windows.h"

static _Atomic int startup_failure[2], terminal_failure[2], invalidations[2];
static _Atomic int release_invalidations;
static _Atomic int default_device[2], opens[2], last_opened[2], live_clients;
static _Atomic int live_devices, com_threads, silent, readings, nonzero, failures;
static thrd_t event_thread;
static int16_t samples[960];
static _Thread_local BYTE render_samples[8192];

static HRESULT CoInitializeEx(void *arg, unsigned flags) {
    (void)arg; (void)flags; ++com_threads; return S_OK;
}
static void CoUninitialize(void) { --com_threads; }
static HRESULT CoCreateInstance(const GUID *cls, void *outer, unsigned context,
    const GUID *iid, void **result) {
    (void)cls; (void)outer; (void)context; (void)iid;
    *result = calloc(1, sizeof(IMMDeviceEnumerator)); return *result ? S_OK : E_FAIL;
}
static HRESULT IMMDeviceEnumerator_GetDefaultAudioEndpoint(IMMDeviceEnumerator *enumerator,
    int input, int role, IMMDevice **device) {
    (void)enumerator; assert(role == eCommunications);
    *device = calloc(1, sizeof(**device)); assert(*device);
    (*device)->input = input; (*device)->id = default_device[input];
    ++live_devices; return S_OK;
}
static HRESULT IMMDeviceEnumerator_GetDevice(IMMDeviceEnumerator *enumerator,
    LPCWSTR id, IMMDevice **device) {
    (void)enumerator;
    *device = calloc(1, sizeof(**device)); assert(*device);
    /* Test explicit endpoint IDs carry direction, as Windows IDs do. */
    (*device)->input = id[0] == L'i'; (*device)->id = (int)wcstol(id + 1, NULL, 10);
    ++live_devices; return S_OK;
}
static HRESULT IMMDevice_GetId(IMMDevice *device, LPWSTR *id) {
    *id = calloc(16, sizeof(wchar_t)); assert(*id);
    swprintf(*id, 16, L"%c%d", device->input ? L'i' : L'o', device->id);
    return S_OK;
}
static void release_device(IMMDevice *device) { if (device) { --live_devices; free(device); } }
#undef IMMDevice_Release
#define IMMDevice_Release release_device
static HRESULT IMMDevice_Activate(IMMDevice *device, const GUID *iid,
    unsigned context, void *parameters, void **result) {
    (void)iid; (void)context; (void)parameters;
    IAudioClient *client = calloc(1, sizeof(*client)); assert(client);
    client->input = device->input; client->id = device->id;
    ++opens[client->input]; last_opened[client->input] = client->id;
    ++live_clients; *result = client; return S_OK;
}
static void release_client(IAudioClient *client) { if (client) { --live_clients; free(client); } }
#undef IAudioClient_Release
#define IAudioClient_Release release_client
static HRESULT IAudioClient_GetMixFormat(IAudioClient *client, WAVEFORMATEX **format) {
    (void)client; *format = calloc(1, sizeof(**format)); return *format ? S_OK : E_FAIL;
}
static HRESULT IAudioClient_Initialize(IAudioClient *client, int mode, DWORD flags,
    int64_t duration, int64_t periodicity, const WAVEFORMATEX *format, void *session) {
    (void)mode; (void)flags; (void)duration; (void)periodicity; (void)session;
    client->format = *format;
    return startup_failure[client->input] ? E_ACCESSDENIED : S_OK;
}
static HRESULT IAudioClient_GetService(IAudioClient *client, const GUID *iid, void **service) {
    (void)iid; *service = client; return S_OK;
}
static HRESULT IAudioClient_GetBufferSize(IAudioClient *client, uint32_t *frames) {
    (void)client; *frames = 4096; return S_OK;
}
static HRESULT IAudioClient_Start(IAudioClient *client) { (void)client; return S_OK; }
static HRESULT IAudioClient_Stop(IAudioClient *client) { (void)client; return S_OK; }
static HRESULT stream_result(IAudioClient *client) {
    if (terminal_failure[client->input]) return E_ACCESSDENIED;
    if (invalidations[client->input] > 0) {
        --invalidations[client->input]; return AUDCLNT_E_DEVICE_INVALIDATED;
    }
    return S_OK;
}
static HRESULT IAudioClient_GetCurrentPadding(IAudioClient *client, uint32_t *frames) {
    *frames = 0; sys_msleep(1); return stream_result(client);
}
static HRESULT IAudioCaptureClient_GetNextPacketSize(IAudioClient *client, uint32_t *frames) {
    *frames = 960; sys_msleep(1); return stream_result(client);
}
static HRESULT IAudioCaptureClient_GetBuffer(IAudioClient *client, BYTE **data,
    uint32_t *frames, DWORD *flags, void *position, void *counter) {
    (void)client; (void)position; (void)counter;
    *data = silent ? NULL : (BYTE *)samples; *frames = 960;
    *flags = silent ? AUDCLNT_BUFFERFLAGS_SILENT : 0;
    return S_OK;
}
static HRESULT IAudioCaptureClient_ReleaseBuffer(IAudioClient *client, uint32_t frames) {
    (void)client; (void)frames;
    if (release_invalidations > 0) { --release_invalidations; return AUDCLNT_E_DEVICE_INVALIDATED; }
    return S_OK;
}
static HRESULT IAudioRenderClient_GetBuffer(IAudioClient *client, uint32_t frames, BYTE **data) {
    (void)client; assert(frames <= 4096); *data = render_samples; return S_OK;
}
static HRESULT IAudioRenderClient_ReleaseBuffer(IAudioClient *client, uint32_t frames, DWORD flags) {
    (void)client; (void)frames; (void)flags; return S_OK;
}
static int MultiByteToWideChar(unsigned page, DWORD flags, const char *source,
    int length, LPWSTR output, int capacity) {
    (void)page; (void)flags;
    for (int i = 0; output && i < length && i < capacity; ++i) output[i] = (wchar_t)source[i];
    return length;
}
static int WideCharToMultiByte(unsigned page, DWORD flags, LPCWSTR source,
    int length, char *output, int capacity, void *fallback, void *used) {
    (void)page; (void)flags; (void)fallback; (void)used;
    if (length < 0) length = (int)wcslen(source) + 1;
    for (int i = 0; output && i < length && i < capacity; ++i) output[i] = (char)source[i];
    return length;
}

/* Force the start worker to win exactly between pending-read and timeout CAS. */
static RE_ATOMIC int *boundary_status;
static unsigned boundary_clock_calls;
static uint64_t worker_clock(void) {
    if (!boundary_status) return tmr_jiffies();
    if (!boundary_clock_calls++) return 0;
    re_atomic_store(boundary_status, 0, re_memory_order_release);
    return 5000;
}
#define tmr_jiffies worker_clock
#include SIP_WASAPI_SRC
#include SIP_WASAPI_PLAY
#include SIP_WASAPI_UTIL
#undef tmr_jiffies

static void read_samples(struct auframe *frame, void *arg) {
    (void)arg; ++readings;
    const int16_t *values = frame->sampv;
    for (size_t i = 0; i < frame->sampc; ++i) if (values[i]) { ++nonzero; break; }
}
static void write_samples(struct auframe *frame, void *arg) {
    (void)arg; memset(frame->sampv, 0, frame->sampc * 2);
}
static void failed(int error, const char *message, void *arg) {
    (void)message; assert(error);
    assert(thrd_equal(thrd_current(), event_thread));
    ++failures;
    /* Real upstream error callbacks can synchronously close/destroy a source. */
    if (arg) {
        struct ausrc_st **owner = arg;
        *owner = mem_deref(*owner);
    }
}
static void end_poll(void *arg) { (void)arg; re_cancel(); }
static void poll_for(unsigned milliseconds) {
    struct tmr timer;
    tmr_init(&timer);
    tmr_start(&timer, milliseconds, end_poll, NULL);
    assert(!re_main(NULL));
    tmr_cancel(&timer);
}
static void wait_count(_Atomic int *value, int target) {
    uint64_t deadline = tmr_jiffies() + 2000;
    while (*value < target) { assert(tmr_jiffies() < deadline); poll_for(10); }
}
static void no_resources(void) {
    assert(live_clients == 0 && live_devices == 0 && com_threads == 0);
}

int main(void) {
    struct ausrc source = {0};
    struct auplay player = {0};
    struct ausrc_prm capture = {.srate = 48000, .ch = 1, .ptime = 20, .fmt = AUFMT_S16LE};
    struct auplay_prm render = {.srate = 48000, .ch = 1, .ptime = 20, .fmt = AUFMT_S16LE};
    struct ausrc_st *input = NULL;
    struct auplay_st *output = NULL;
    assert(!libre_init());
    event_thread = thrd_current();
    RE_ATOMIC int boundary = EINPROGRESS;
    boundary_status = &boundary;
    assert(!wasapi_wait_start(&boundary));
    boundary_status = NULL;
    re_atomic_store(&boundary, ETIMEDOUT, re_memory_order_release);
    assert(!wasapi_started(&boundary));
    assert((int)re_atomic_load(&boundary, re_memory_order_acquire) == ETIMEDOUT);
    startup_failure[eCapture] = 1;
    int error = wasapi_src_alloc(&input, &source, &capture, "default", read_samples, failed, NULL);
    assert(error != 0 && input == NULL); /* Denial must fail the accepted-start handshake. */
    startup_failure[eCapture] = 0;
    startup_failure[eRender] = 1;
    error = wasapi_play_alloc(&output, &player, &render, "default", write_samples, NULL);
    assert(error != 0 && output == NULL);
    startup_failure[eRender] = 0;
    no_resources();

    /* A SILENT buffer may have a null data pointer and still carry frames. */
    for (size_t i = 0; i < 960; ++i) samples[i] = 123;
    silent = 1;
    assert(!wasapi_src_alloc(&input, &source, &capture, "default", read_samples, failed, NULL));
    wait_count(&readings, 3);
    input = mem_deref(input);
    assert(nonzero == 0);
    no_resources();
    silent = 0;

    /* Default streams follow the communications role, even without unplugging. */
    assert(!wasapi_src_alloc(&input, &source, &capture, "default", read_samples, failed, NULL));
    int before = opens[eCapture];
    default_device[eCapture] = 2;
    wait_count(&opens[eCapture], before + 1);
    assert(last_opened[eCapture] == 2 && failures == 0);
    input = mem_deref(input);
    no_resources();

    /* Explicit endpoints recover to the SAME ID, never the new default. */
    assert(!wasapi_src_alloc(&input, &source, &capture, "i7", read_samples, failed, &input));
    before = opens[eCapture];
    default_device[eCapture] = 10;
    invalidations[eCapture] = 1;
    wait_count(&opens[eCapture], before + 1);
    assert(last_opened[eCapture] == 7);
    terminal_failure[eCapture] = 1;
    wait_count(&failures, 1);
    assert(input == NULL);
    no_resources();
    terminal_failure[eCapture] = 0;

    /* Persistent invalidation is bounded, and its one callback owns cleanup. */
    assert(!wasapi_src_alloc(&input, &source, &capture, "i7", read_samples, failed, &input));
    release_invalidations = 100;
    uint64_t deadline = tmr_jiffies() + 6500;
    while (failures < 2) { assert(tmr_jiffies() < deadline); poll_for(10); }
    assert(input == NULL && failures == 2);
    release_invalidations = 0;
    no_resources();

    /* Rendering recovers and exposes terminal status independently per player. */
    assert(!wasapi_play_alloc(&output, &player, &render, "default", write_samples, NULL));
    before = opens[eRender];
    default_device[eRender] = 3;
    wait_count(&opens[eRender], before + 1);
    assert(last_opened[eRender] == 3);
    before = opens[eRender];
    invalidations[eRender] = 1;
    wait_count(&opens[eRender], before + 1);
    assert(!wasapi_play_error(output));
    terminal_failure[eRender] = 1;
    deadline = tmr_jiffies() + 2000;
    while (!wasapi_play_error(output)) { assert(tmr_jiffies() < deadline); poll_for(10); }
    output = mem_deref(output);
    terminal_failure[eRender] = 0;
    no_resources();

    /* Destruction during the retry delay cancels promptly and always joins. */
    assert(!wasapi_play_alloc(&output, &player, &render, "o9", write_samples, NULL));
    before = opens[eRender];
    invalidations[eRender] = 100;
    wait_count(&opens[eRender], before + 1);
    uint64_t stopped = tmr_jiffies();
    output = mem_deref(output);
    assert(tmr_jiffies() - stopped < 1000);
    no_resources();
    libre_close();
    puts("WASAPI_WORKERS_OK startup, silence, same-device/default recovery, terminal errors, bounded retries and joined shutdown");
    return 0;
}
