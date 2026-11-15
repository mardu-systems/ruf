/* Exercise the production queue handler without opening an OS audio device.
 * Only the external ua_answer result is injected; ringtone ownership uses libre. */
/* Keep assertions enabled in Release, before baresip's warning macro is defined. */
#undef NDEBUG
#include <assert.h>
#define ua_answer test_ua_answer
#include "../bridge.c"

static int answer_result;
static unsigned ring_destructions, answer_failures, errors, failed_dials;
static uint64_t failed_dial_generation;
static unsigned activation_failures;

int test_ua_answer(struct ua *ua, struct call *call, enum vidmode mode) {
    assert(ua && call && mode == VIDMODE_OFF);
    return answer_result;
}
int sip_audio_device_available(const char *id, int input) {
    (void)id; (void)input;
    return 0;
}
void sip_network_reset(void) {}
unsigned sip_network_generation(void) { return 0; }
int sip_network_poll(bool active) { (void)active; return 0; }

static void ring_destroyed(void *arg) { (void)arg; ++ring_destructions; }
static void observed(const struct sip_event *event, void *arg) {
    (void)arg;
    if (event->kind == EV_ANSWER_FAILED) ++answer_failures;
    if (event->kind == EV_ERROR) ++errors;
    if (event->kind == EV_ACCOUNT_REJECTED) { assert(event->code == EBUSY); ++activation_failures; }
    if (event->kind == EV_DIAL_FAILED) {
        ++failed_dials;
        failed_dial_generation = event->call_generation;
    }
}
static void execute(struct engine *engine, int operation) {
    struct request *request = calloc(1, sizeof(*request));
    assert(request);
    request->operation = operation;
    if (operation == OP_DISABLE || operation == OP_ENABLE) request->account_id = 77;
    request->call_generation = engine->current->generation;
    queue_handler(operation, request, engine);
}

static unsigned payloads;
static void inspect_payload(int id, void *data, void *arg) {
    struct engine *engine = arg;
    struct request *r = data;
    assert(id == r->operation);
    remove_pending(engine, r);
    switch (payloads++) {
    case 0:
        assert(id == OP_ACCOUNT && r->account_id == 77 && !r->enabled);
        assert(!strcmp(r->registrar, "pbx.test") && !strcmp(r->username, "100"));
        assert(!strcmp(r->password, "fixture-password"));
        assert(!strcmp(r->monitored_extensions, "101\n102"));
        assert(r->transport == 2 && r->media_security == 1);
        break;
    case 1:
        assert(id == OP_SOUND_RESTORE && r->enabled);
        assert(!strcmp(r->input_device, "mic") && !strcmp(r->output_device, "speaker"));
        assert(!strcmp(r->ring_device, "ring") && !r->password);
        assert(r->ringtone_variant == 2 && r->ringtone_volume == 37);
        break;
    case 2:
        assert(id == OP_HOLD && r->enabled && r->call_generation == 42);
        assert(!r->password && !r->device);
        break;
    case 3:
        assert(id == OP_TEST_MIC && r->call_generation == 43);
        assert(!strcmp(r->device, "mic") && !r->registrar);
        break;
    default:
        assert(id == OP_HANGUP);
    }
    free_request(r);
    if (payloads == 16) re_cancel();
}

static void request_payloads(void) {
    struct engine engine = {0};
    char password[] = "fixture-password", input[] = "mic";
    const struct sip_account_config account = {
        "pbx.test", "100", password, "auth", "proxy.test", "101\n102", 2, 1, 0
    };
    const struct sip_sound_settings sound = { input, "speaker", "ring", 1, 2, 37 };
    LOCK_INIT(&engine.lock);
    assert(!libre_init());
    assert(!mqueue_alloc(&engine.queue, inspect_payload, &engine));
    engine.running = true;
    assert(!sip_engine_account(&engine, OP_ACCOUNT, 77, &account));
    assert(!sip_engine_sound(&engine, 1, &sound));
    assert(!sip_engine_command(&engine, OP_HOLD, 77, 42, NULL, 1));
    assert(!sip_engine_audio(&engine, OP_TEST_MIC, 43, input));
    /* All inputs are borrowed only until return, including the secret. */
    memset(password, 'x', sizeof(password) - 1);
    input[0] = 'x';
    for (unsigned i = 4; i < 16; ++i)
        assert(!sip_engine_command(&engine, OP_HANGUP, 77, 42, NULL, 0));
    assert(sip_engine_command(&engine, OP_HANGUP, 77, 42, NULL, 0) == EBUSY);
    /* Stop must succeed even with a saturated queue, without an allocation. */
    assert(!sip_engine_stop(&engine) && engine.stop_requested);
    assert(sip_engine_command(&engine, OP_HANGUP, 77, 42, NULL, 0) == ENOTCONN);
    assert(!re_main(NULL));
    assert(payloads == 16 && !engine.pending);
    mem_deref(engine.queue);
    libre_close();
    LOCK_CLOSE(&engine.lock);
}

int main(void) {
    struct engine engine = {0};
    LOCK_INIT(&engine.lock);
    engine.audio_mode = 1;
    engine.callback = observed;
    engine.current = engine.selected = &engine.dialogs[0];
    engine.current->generation = 42;
    /* Opaque sentinels are only inspected by the injected answer function. */
    engine.current->call = (struct call *)&engine;
    engine.current->ua = (struct ua *)&engine;
    engine.ring = mem_zalloc(1, ring_destroyed);
    assert(engine.ring);
    answer_result = EAGAIN;
    execute(&engine, OP_ANSWER);
    assert(answer_failures == 1);
    assert(engine.ring && ring_destructions == 0);
    assert(!engine.current->answered);

    answer_result = 0;
    execute(&engine, OP_ANSWER);
    assert(!engine.ring && ring_destructions == 1);
    assert(engine.current->answered);

    /* Another already-queued hangup of the same closed generation is harmless. */
    engine.current->call = NULL;
    engine.current->ua = NULL;
    execute(&engine, OP_HANGUP);
    assert(errors == 0);
    /* An outgoing attempt losing the slot race must only fail its own token. */
    engine.current->call = (struct call *)&engine;
    engine.current->ua = (struct ua *)&engine;
    struct request *dial = calloc(1, sizeof(*dial));
    assert(dial);
    dial->operation = OP_DIAL;
    dial->call_generation = 43;
    queue_handler(OP_DIAL, dial, &engine);
    assert(failed_dials == 1 && failed_dial_generation == 43);
    assert(engine.current->call && engine.current->generation == 42);
    assert(errors == 0);
    /* An incoming/dialing call winning after Rust validation must reject both
       queued toggles before touching the owned UA or account identity. */
    engine.accounts[0].id = 77;
    engine.accounts[0].ua = (struct ua *)&engine;
    execute(&engine, OP_DISABLE);
    execute(&engine, OP_ENABLE);
    assert(activation_failures == 2);
    assert(engine.accounts[0].id == 77 && engine.accounts[0].ua == (struct ua *)&engine);
    assert(engine.current->call && engine.current->generation == 42);
    LOCK_CLOSE(&engine.lock);
    request_payloads();
    puts("call_actions: answer failure retains ringtone; queued duplicate hangup and losing dial preserve the current call");
    return 0;
}
