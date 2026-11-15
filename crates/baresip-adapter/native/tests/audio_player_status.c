/* Exercise the patched SDK status API with allocated fake players. No devices. */
#undef NDEBUG
#include <assert.h>
#include <stdio.h>
#include <re.h>
#include <rem.h>
#include <baresip.h>

struct auplay_st { int error; };
static struct auplay_st *last_state;

static int fake_alloc(struct auplay_st **stp, const struct auplay *ap,
                      struct auplay_prm *prm, const char *device,
                      auplay_write_h *wh, void *arg) {
    (void)ap; (void)prm; (void)device; (void)wh; (void)arg;
    *stp = last_state = mem_zalloc(sizeof(**stp), NULL);
    return *stp ? 0 : ENOMEM;
}
static int fake_error(const struct auplay_st *st) { return st->error; }

int main(void) {
    struct auplay *plain = NULL, *reporting = NULL;
    struct auplay_st *state = NULL;
    struct auplay_prm prm = {8000, 1, 40, AUFMT_S16LE};
    struct player *player = NULL;
    struct play *play = NULL;
    struct mbuf *tone;
    assert(!libre_init());
    assert(!auplay_register(&plain, baresip_auplayl(), "plain", fake_alloc));
    assert(!auplay_register(&reporting, baresip_auplayl(), "reporting", fake_alloc));
    assert(!plain->errorh && !reporting->errorh);
    reporting->errorh = fake_error;
    assert(!auplay_alloc(&state, baresip_auplayl(), "reporting", &prm,
                         NULL, NULL, NULL));
    assert(!auplay_error(NULL, state));
    assert(!auplay_error(reporting, NULL));
    assert(!auplay_error(plain, state));
    assert(!auplay_error(reporting, state));
    state->error = EIO;
    assert(auplay_error(reporting, state) == EIO);
    mem_deref(state);
    assert(!play_error(NULL));
    assert(!audio_play_error(NULL));
    assert(!play_init(&player));
    tone = mbuf_alloc(640);
    assert(tone && !mbuf_fill(tone, 0, 640));
    tone->pos = 0;
    /* An explicit backend must be retained, not the first registered backend. */
    assert(!play_tone(&play, player, tone, 8000, 1, -1, "reporting", NULL));
    assert(!play_error(play));
    last_state->error = EPIPE;
    assert(play_error(play) == EPIPE);
    play = mem_deref(play);
    /* The default selection still accepts a backend without a status hook. */
    assert(!play_tone(&play, player, tone, 8000, 1, -1, NULL, NULL));
    last_state->error = EIO;
    assert(!play_error(play));
    mem_deref(play);
    mem_deref(tone);
    mem_deref(player);
    mem_deref(reporting);
    mem_deref(plain);
    libre_close();
    puts("AUDIO_PLAYER_STATUS_OK: optional hooks and selected backend errors");
    return 0;
}
