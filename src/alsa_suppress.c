#include <alsa/asoundlib.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

static void filtered_error_handler(const char *file, int line,
                                   const char *function, int err,
                                   const char *fmt, ...) {
    /* Suppress ALSA poll() errors with errno -4 (EINTR).
       These are harmless and spam stderr during audio playback. */
    if (function && strcmp(function, "poll") == 0 && err == -4)
        return;

    /* Forward all other errors to stderr (default ALSA behavior) */
    va_list ap;
    va_start(ap, fmt);
    fprintf(stderr, "ALSA lib %s:%i:(%s) ", file, line, function);
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, "\n");
    va_end(ap);
}

void suppress_alsa_errors(void) {
    snd_lib_error_set_handler(filtered_error_handler);
}
