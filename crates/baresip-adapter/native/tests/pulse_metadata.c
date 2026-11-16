/* Inject the documented libpulse error callback into exact staged module code. */
#include <assert.h>
#include <stdio.h>
#ifdef TEST_PLAYER
#define pulse_player_alloc tested_player_alloc
#define pulse_player_init tested_player_init
#define stream_write_cb tested_write_cb
#else
#define pulse_recorder_alloc tested_recorder_alloc
#define pulse_recorder_init tested_recorder_init
#define stream_read_cb tested_read_cb
#endif
#include PULSE_SOURCE

int main(void) {
    struct list devices = LIST_INIT;
    dev_list_cb(NULL, NULL, -1, &devices);
    dev_list_cb(NULL, NULL, 1, &devices);
    assert(!list_head(&devices));
    puts("PULSE_METADATA_OK: server-error and terminal callbacks accept null info");
    return 0;
}
