// The C ABI every toy's static library exports (toy-kit's `ios_shell!`, toys/crates/toy-kit/src/lib.rs).
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
typedef struct ToyShell ToyShell;
typedef struct ToySynth ToySynth;
ToyShell *toy_new(void *layer, uint32_t width, uint32_t height, double scale, const char *assets);
void toy_resize(ToyShell *shell, uint32_t width, uint32_t height, double scale);
void toy_frame(ToyShell *shell, double t);
void toy_pointer(ToyShell *shell, uint64_t id, uint32_t phase, double x, double y, double t);
void toy_shaken(ToyShell *shell);
bool toy_take_haptic(ToyShell *shell);
void toy_set_night(ToyShell *shell, bool on);
void toy_set_reduce_motion(ToyShell *shell, bool on);
void toy_store(ToyShell *shell, const char *path);
void toy_pause(ToyShell *shell);
ToySynth *toy_synth(ToyShell *shell);
void toy_audio(ToySynth *synth, float *out, uint32_t frames, uint32_t channels, double rate);
void toy_synth_free(ToySynth *synth);
void toy_free(ToyShell *shell);
