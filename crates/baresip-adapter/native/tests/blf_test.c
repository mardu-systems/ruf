/* Native BLF boundary: invalid target lists must never reach SIP allocation. */
#undef NDEBUG
#include <assert.h>
#include <stdio.h>
#include <string.h>
#define sip_treply test_reply
#include "../blf.c"

static unsigned replies, invalidations, bodies;
static uint16_t status;
static int require_resync;

int test_reply(struct sip_strans **transaction, struct sip *sip, const struct sip_msg *message,
               uint16_t code, const char *reason) {
    (void)transaction; (void)sip; (void)message; (void)reason;
    ++replies;
    status = code;
    return 0;
}

static int observed(uint32_t account, const char *extension, const char *body, void *arg) {
    assert(account == 7 && !strcmp(extension, "1001") && !arg);
    if (body) { ++bodies; assert(!strcmp(body, "<dialog-info/>")); }
    else ++invalidations;
    return require_resync;
}

static void notification(struct blf_target *target, const char *state, const char *type,
                         const char *body) {
    struct mbuf *buffer = mbuf_alloc(1024);
    struct sip_msg *message = NULL;
    assert(buffer);
    assert(!mbuf_printf(buffer,
        "NOTIFY sip:1000@127.0.0.1 SIP/2.0\r\n"
        "Via: SIP/2.0/UDP 127.0.0.1:5060;branch=z9hG4bK-blf\r\n"
        "To: <sip:1000@127.0.0.1>;tag=local\r\n"
        "From: <sip:1001@127.0.0.1>;tag=remote\r\n"
        "Call-ID: synthetic-blf\r\nCSeq: 1 NOTIFY\r\n"
        "Event: dialog\r\nSubscription-State: %s\r\n"
        "Content-Type: %s\r\nContent-Length: %zu\r\n\r\n%s",
        state, type, strlen(body), body));
    buffer->pos = 0;
    assert(!sip_msg_decode(&message, buffer));
    notified(NULL, message, target);
    mem_deref(message);
    mem_deref(buffer);
}

static void notification_validity_and_resync(void) {
    struct sip_blf owner = { .account_id = 7, .registered = true, .handler = observed };
    struct blf_target target = { .owner = &owner, .extension = "1001" };
    tmr_init(&target.retry);
    tmr_init(&target.expiry);
    notification(&target, "active;expires=120", "application/dialog-info+xml;charset=UTF-8", "<dialog-info/>");
    assert(replies == 1 && status == 200 && bodies == 1 && !invalidations);
    assert(tmr_get_expire(&target.expiry) > 119000 && !tmr_isrunning(&target.retry));
    /* Quiet, still-valid state is retained until negotiated expiry. */
    assert(tmr_get_expire(&target.expiry) > 119000);
    require_resync = 1;
    notification(&target, "active;expires=120", "application/dialog-info+xml", "<dialog-info/>");
    assert(replies == 2 && status == 200 && bodies == 2 && invalidations == 1);
    assert(tmr_isrunning(&target.retry) && !tmr_isrunning(&target.expiry));
    require_resync = 0;
    notification(&target, "active;expires=120", "application/dialog-info+xml", "<dialog-info/>");
    assert(!tmr_isrunning(&target.retry) && tmr_isrunning(&target.expiry));
    tmr_cancel(&target.expiry);
    tmr_cancel(&target.retry);
    notification(&target, "pending;expires=120", "application/dialog-info+xml", "<dialog-info/>");
    assert(status == 200 && bodies == 3 && invalidations == 2);
    notification(&target, "active;expires=120", "application/pidf+xml", "<dialog-info/>");
    assert(status == 415 && bodies == 3 && invalidations >= 3);
    assert(tmr_isrunning(&target.retry));
    tmr_cancel(&target.retry);
    notification(&target, "active;expires=120", "application/dialog-info+xml", "");
    assert(status == 400 && tmr_isrunning(&target.retry));
    tmr_cancel(&target.retry);
    struct sipevent_substate ended = { .state = SIPEVENT_TERMINATED, .reason = SIPEVENT_PROBATION };
    pl_set_str(&ended.retry_after, "120");
    closed(0, NULL, &ended, &target);
    assert(tmr_get_expire(&target.retry) > 119000);
    tmr_cancel(&target.retry);
    owner.retired = true;
    unsigned prior = invalidations;
    notification(&target, "active;expires=120", "application/dialog-info+xml", "<dialog-info/>");
    assert(invalidations == prior && bodies == 3);
    tmr_cancel(&target.expiry);
}

static void subscribe_retry_after(void) {
    struct pl invalid;
    for (size_t i = 0; i < 4; ++i) {
        const char *values[] = { "-1", "120invalid", "4294967296", "999999999999999999999999999999999999999" };
        pl_set_str(&invalid, values[i]);
        assert(!retry_after_ms(&invalid));
    }
    struct sip_blf owner = { .account_id = 7, .registered = true, .handler = observed };
    struct blf_target target = { .owner = &owner, .extension = "1001" };
    struct mbuf *buffer = mbuf_alloc(1024);
    struct sip_msg *message = NULL;
    assert(buffer);
    assert(!mbuf_printf(buffer,
        "SIP/2.0 503 Service Unavailable\r\n"
        "Via: SIP/2.0/UDP 127.0.0.1:5060;branch=z9hG4bK-backoff\r\n"
        "To: <sip:1000@127.0.0.1>;tag=local\r\n"
        "From: <sip:1001@127.0.0.1>;tag=remote\r\n"
        "Call-ID: synthetic-blf\r\nCSeq: 1 SUBSCRIBE\r\n"
        "Retry-After: 120 (maintenance);duration=30\r\nContent-Length: 0\r\n\r\n"));
    buffer->pos = 0;
    assert(!sip_msg_decode(&message, buffer));
    tmr_init(&target.retry);
    tmr_init(&target.expiry);
    closed(0, message, NULL, &target);
    assert(tmr_get_expire(&target.retry) > 119000);
    tmr_cancel(&target.retry);
    mem_deref(message);
    mem_deref(buffer);
}


int main(void) {
    assert(!libre_init());
    notification_validity_and_resync();
    subscribe_retry_after();
    assert(!sip_blf_validate_targets(""));
    assert(!sip_blf_validate_targets("1000\n1001\n001"));
    assert(sip_blf_validate_targets("1000\n1000"));
    assert(sip_blf_validate_targets("1000\n"));
    assert(sip_blf_validate_targets("\n1000"));
    assert(sip_blf_validate_targets("1000\r\n1001"));
    assert(sip_blf_validate_targets("sip:1000@pbx.test"));
    assert(sip_blf_validate_targets("1000;transport=tcp"));
    assert(sip_blf_validate_targets("123456789012345678901234567890123"));
    char targets[1056] = {0};
    for (unsigned i = 0; i < 32; ++i) {
        char number[34];
        snprintf(number, sizeof(number), "%s%u", i ? "\n" : "", i);
        strcat(targets, number);
    }
    assert(!sip_blf_validate_targets(targets));
    strcat(targets, "\n32");
    assert(sip_blf_validate_targets(targets));
    assert(sip_blf_validate_targets(NULL));
    libre_close();
    puts("blf: rejects duplicate, injected, oversized and excess targets");
    return 0;
}
