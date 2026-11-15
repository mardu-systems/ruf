/* SIP dialog-event subscriptions, independent of audio and the webview. */
#include <string.h>
#include <re.h>
#include <baresip.h>
#include "blf.h"

enum {
    BLF_TARGETS = 32,
    BLF_DIGITS = 32,
    BLF_TARGET_BYTES = BLF_TARGETS * (BLF_DIGITS + 1),
    BLF_BODY_BYTES = 65536,
    BLF_EXPIRES = 300,
    BLF_INITIAL_WAIT_MS = 10000,
    BLF_MAX_BACKOFF_MS = 60000,
};

struct sip_blf;
struct blf_target {
    struct sip_blf *owner;
    struct sipsub *sub;
    struct tmr retry, expiry;
    unsigned failures;
    char extension[BLF_DIGITS + 1];
};

struct sip_blf {
    struct ua *ua;
    uint32_t account_id;
    int transport;
    bool registered, retired;
    char registrar[256];
    sip_blf_handler *handler;
    void *arg;
    size_t count;
    struct blf_target targets[BLF_TARGETS];
};

int sip_blf_validate_targets(const char *targets) {
    const char *starts[BLF_TARGETS];
    size_t lengths[BLF_TARGETS], count = 0, position = 0;
    if (!targets) return EINVAL;
    if (!targets[0]) return 0;
    for (;;) {
        if (count == BLF_TARGETS) return EINVAL;
        size_t start = position;
        while (targets[position] >= '0' && targets[position] <= '9') {
            if (++position - start > BLF_DIGITS) return EINVAL;
        }
        size_t length = position - start;
        if (!length || (targets[position] && targets[position] != '\n')) return EINVAL;
        for (size_t i = 0; i < count; ++i)
            if (lengths[i] == length && !memcmp(starts[i], targets + start, length)) return EINVAL;
        starts[count] = targets + start;
        lengths[count++] = length;
        if (!targets[position]) return 0;
        if (++position >= BLF_TARGET_BYTES) return EINVAL;
    }
}

static void unknown(struct blf_target *target) {
    struct sip_blf *blf = target->owner;
    if (!blf->retired) (void)blf->handler(blf->account_id, target->extension, NULL, blf->arg);
}

static unsigned retry_delay(struct blf_target *target) {
    unsigned delay = 1000u << min(target->failures, 6u);
    if (target->failures < 6) ++target->failures;
    return min(delay, (unsigned)BLF_MAX_BACKOFF_MS);
}

static void subscribe(void *arg);

static void expired(void *arg) {
    struct blf_target *target = arg;
    unknown(target);
    /* A negotiated lifetime elapsed without renewal evidence, or initial
       SUBSCRIBE never delivered its required NOTIFY. Recreate out of callback. */
    tmr_start(&target->retry, retry_delay(target), subscribe, target);
}

static int authenticate(char **username, char **password, const char *realm, void *arg) {
    return account_auth(arg, username, password, realm);
}

/* Parse delta-seconds without overflow; Retry-After permits comments/parameters. */
static uint64_t retry_after_ms(const struct pl *value) {
    uint64_t seconds = 0;
    size_t i = 0;
    if (!value || !value->p) return 0;
    while (i < value->l && value->p[i] >= '0' && value->p[i] <= '9') {
        unsigned digit = (unsigned)(value->p[i++] - '0');
        if (seconds > (UINT32_MAX - digit) / 10u) return 0;
        seconds = seconds * 10u + digit;
    }
    if (!i || (i < value->l && value->p[i] != ' ' && value->p[i] != '\t'
            && value->p[i] != '(' && value->p[i] != ';')) return 0;
    return seconds * 1000;
}

static void closed(int err, const struct sip_msg *message,
                   const struct sipevent_substate *state, void *arg) {
    struct blf_target *target = arg;
    (void)err;
    target->sub = mem_deref(target->sub);
    tmr_cancel(&target->expiry);
    if (!target->owner->registered || target->owner->retired) return;
    unknown(target);
    uint64_t wait = retry_delay(target);
    if (message && (message->scode == 403 || message->scode == 404 || message->scode == 489))
        wait = BLF_MAX_BACKOFF_MS;
    if (message) {
        const struct sip_hdr *retry = sip_msg_hdr(message, SIP_HDR_RETRY_AFTER);
        if (retry) wait = max(wait, retry_after_ms(&retry->val));
    }
    if (state && pl_isset(&state->retry_after))
        wait = max(wait, retry_after_ms(&state->retry_after));
    if (state && (state->reason == SIPEVENT_REJECTED || state->reason == SIPEVENT_NORESOURCE))
        wait = max(wait, (uint64_t)BLF_MAX_BACKOFF_MS);
    tmr_start(&target->retry, wait, subscribe, target);
}

static bool content_type(const struct sip_msg *message) {
    const struct sip_hdr *header = sip_msg_hdr(message, SIP_HDR_CONTENT_TYPE);
    if (!header) return false;
    struct pl type = header->val;
    const char *end = memchr(type.p, ';', type.l);
    if (end) type.l = (size_t)(end - type.p);
    while (type.l && (type.p[type.l - 1] == ' ' || type.p[type.l - 1] == '\t')) --type.l;
    return !pl_strcasecmp(&type, "application/dialog-info+xml");
}

static void notified(struct sip *sip, const struct sip_msg *message, void *arg) {
    struct blf_target *target = arg;
    struct sip_blf *blf = target->owner;
    struct sipevent_substate state;
    const struct sip_hdr *header = sip_msg_hdr(message, SIP_HDR_SUBSCRIPTION_STATE);
    bool active = blf->registered && !blf->retired && header &&
        !sipevent_substate_decode(&state, &header->val) && state.state == SIPEVENT_ACTIVE;
    size_t length = mbuf_get_left(message->mb);
    int resync = 0;
    if (!active) {
        unknown(target);
        (void)sip_treply(NULL, sip, message, 200, "OK");
        return;
    }
    if (!content_type(message) || !length || length > BLF_BODY_BYTES ||
            memchr(mbuf_buf(message->mb), '\0', length)) {
        unknown(target);
        (void)sip_treply(NULL, sip, message, content_type(message) ? 400 : 415,
                         content_type(message) ? "Bad Dialog Body" : "Unsupported Media Type");
        resync = 1;
    } else {
        char *body = mem_alloc(length + 1, NULL);
        if (!body) {
            unknown(target);
            (void)sip_treply(NULL, sip, message, 500, "Allocation Failed");
            resync = 1;
        } else {
            memcpy(body, mbuf_buf(message->mb), length);
            body[length] = '\0';
            resync = blf->handler(blf->account_id, target->extension, body, blf->arg);
            mem_deref(body);
            (void)sip_treply(NULL, sip, message, 200, "OK");
        }
    }
    if (resync) {
        unknown(target);
        /* Do not destroy a subscription inside its active notify callback. */
        tmr_cancel(&target->expiry);
        tmr_start(&target->retry, retry_delay(target), subscribe, target);
        return;
    }
    target->failures = 0;
    tmr_cancel(&target->retry);
    uint64_t expires = pl_isset(&state.expires) ? pl_u32(&state.expires) : BLF_EXPIRES;
    tmr_start(&target->expiry, expires * 1000, expired, target);
}

static void subscribe(void *arg) {
    struct blf_target *target = arg;
    struct sip_blf *blf = target->owner;
    tmr_cancel(&target->retry);
    tmr_cancel(&target->expiry);
    target->sub = mem_deref(target->sub);
    if (!blf->registered || blf->retired) return;
    unknown(target);
    const char *routes[] = { ua_outbound(blf->ua) };
    const char *transport = blf->transport == 2 ? "tls" : blf->transport == 1 ? "tcp" : "udp";
    char uri[384];
    int result = re_snprintf(uri, sizeof(uri), "sip:%s@%s;transport=%s",
                             target->extension, blf->registrar, transport);
    int error = result < 0 || (size_t)result >= sizeof(uri) ? EINVAL :
        sipevent_subscribe(&target->sub, uag_sipevent_sock(), uri, NULL,
            account_aor(ua_account(blf->ua)), "dialog", NULL, BLF_EXPIRES,
            ua_cuser(blf->ua), routes, routes[0] ? 1 : 0,
            authenticate, ua_account(blf->ua), true, NULL, notified, closed, target,
            "Accept: application/dialog-info+xml\r\n");
    if (error) tmr_start(&target->retry, retry_delay(target), subscribe, target);
    else tmr_start(&target->expiry, BLF_INITIAL_WAIT_MS, expired, target);
}

static void destroyed(void *arg) {
    struct sip_blf *blf = arg;
    blf->retired = true;
    for (size_t i = 0; i < blf->count; ++i) {
        struct blf_target *target = &blf->targets[i];
        tmr_cancel(&target->retry);
        tmr_cancel(&target->expiry);
        target->sub = mem_deref(target->sub);
    }
    mem_deref(blf->ua);
}

int sip_blf_alloc(struct sip_blf **blfp, struct ua *ua, uint32_t account_id,
                  const char *registrar, int transport, const char *targets,
                  sip_blf_handler *handler, void *arg) {
    if (!blfp || !ua || !account_id || !registrar || !registrar[0] ||
            strlen(registrar) >= 256 || transport < 0 || transport > 2 || !handler ||
            sip_blf_validate_targets(targets)) return EINVAL;
    *blfp = NULL;
    if (!targets[0]) return 0;
    struct sip_blf *blf = mem_zalloc(sizeof(*blf), destroyed);
    if (!blf) return ENOMEM;
    blf->ua = mem_ref(ua);
    blf->account_id = account_id;
    blf->transport = transport;
    blf->handler = handler;
    blf->arg = arg;
    str_ncpy(blf->registrar, registrar, sizeof(blf->registrar));
    const char *start = targets;
    while (*start) {
        const char *end = strchr(start, '\n');
        size_t length = end ? (size_t)(end - start) : strlen(start);
        struct blf_target *target = &blf->targets[blf->count++];
        target->owner = blf;
        memcpy(target->extension, start, length);
        tmr_init(&target->retry);
        tmr_init(&target->expiry);
        start += length + (end ? 1 : 0);
    }
    *blfp = blf;
    return 0;
}

void sip_blf_registered(struct sip_blf *blf) {
    if (!blf || blf->retired || blf->registered) return;
    blf->registered = true;
    for (size_t i = 0; i < blf->count; ++i) subscribe(&blf->targets[i]);
}

void sip_blf_suspend(struct sip_blf *blf) {
    if (!blf || blf->retired) return;
    blf->registered = false;
    for (size_t i = 0; i < blf->count; ++i) {
        struct blf_target *target = &blf->targets[i];
        tmr_cancel(&target->retry);
        tmr_cancel(&target->expiry);
        target->sub = mem_deref(target->sub);
        unknown(target);
    }
}
