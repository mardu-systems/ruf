/* Observe production event-loop routing of independent playback failures. */
#undef NDEBUG
#include <assert.h>
#define play_error test_play_error
#define audio_play_error test_audio_play_error
#define call_audio test_call_audio
#define ua_hangup test_ua_hangup
#include "../bridge.c"

static struct engine *owner;
static const struct play *failed_player;
static const struct audio *failed_audio;
static unsigned test_failures, call_errors, generic_errors, hangups;
static uint32_t failed_account;
static uint64_t failed_generation;
int test_play_error(const struct play *player) { return player && player == failed_player ? EIO : 0; }
int test_audio_play_error(const struct audio *audio) { return audio && audio == failed_audio ? EIO : 0; }
struct audio *test_call_audio(const struct call *call) { return (struct audio *)call; }
void test_ua_hangup(struct ua *ua, struct call *call, uint16_t code, const char *reason) {
    (void)ua; (void)code; (void)reason;
    assert(owner->current->call == call);
    ++hangups;
    /* Model synchronous close; status handling must not touch destroyed audio. */
    owner->current->call = NULL;
}
int sip_audio_device_available(const char *id, int input) { (void)id; (void)input; return 0; }
void sip_network_reset(void) {}
unsigned sip_network_generation(void) { return 0; }
int sip_network_poll(bool active) { (void)active; return 0; }
static void observed(const struct sip_event *event, void *arg) {
    (void)arg;
    if (event->kind == EV_TEST_FAILED) { ++test_failures; failed_generation = event->call_generation; }
    if (event->kind == EV_CALL_ERROR) {
        ++call_errors; failed_account = event->account_id; failed_generation = event->call_generation;
    }
    if (event->kind == EV_ERROR) ++generic_errors;
}
int main(void) {
    struct engine engine = {0};
    assert(!libre_init());
    owner = &engine;
    LOCK_INIT(&engine.lock);
    engine.callback = observed;
    engine.selected = engine.current = &engine.dialogs[0];
    engine.test_kind = 1; engine.test_id = 77;
    engine.test_player = mem_zalloc(1, NULL);
    failed_player = engine.test_player;
    shutdown_tick(&engine);
    assert(test_failures == 1 && failed_generation == 77);
    assert(!engine.test_player && !engine.test_id);
    shutdown_tick(&engine);
    assert(test_failures == 1);

    engine.ring = mem_zalloc(1, NULL);
    failed_player = engine.ring;
    shutdown_tick(&engine);
    assert(!engine.ring && generic_errors == 1);

    engine.dialogs[0].call = (struct call *)&engine.dialogs[0];
    engine.dialogs[0].generation = 10; engine.dialogs[0].account_id = 1;
    engine.dialogs[1].call = (struct call *)&engine.dialogs[1];
    engine.dialogs[1].generation = 11; engine.dialogs[1].account_id = 2;
    failed_audio = (struct audio *)engine.dialogs[1].call;
    shutdown_tick(&engine);
    assert(call_errors == 1 && failed_account == 2 && failed_generation == 11);
    assert(hangups == 1 && engine.dialogs[0].call && !engine.dialogs[1].call);
    assert(engine.current == engine.selected);
    shutdown_tick(&engine);
    assert(call_errors == 1 && hangups == 1);
    tmr_cancel(&engine.shutdown_timer);
    LOCK_CLOSE(&engine.lock);
    libre_close();
    puts("AUDIO_FAILURES_OK owned test/ringtone failures and background call error isolation");
    return 0;
}
