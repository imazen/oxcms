/* Test-only icclib oracle. Input is normalized ICC ink (0=no ink), not percent.
 * Rendering intent is literal 0..3. No black-point compensation is applied.
 * The two lookups use XYZ PCS and retain floating output outside RGB [0,1]. */
#include "icc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (!file || fseek(file, 0, SEEK_END)) return NULL;
    long n = ftell(file);
    if (n <= 0 || n > 64000000 || fseek(file, 0, SEEK_SET)) return NULL;
    void *bytes = malloc((size_t)n);
    if (!bytes || fread(bytes, 1, (size_t)n, file) != (size_t)n) return NULL;
    fclose(file); *length = (size_t)n; return bytes;
}
int main(int argc, char **argv) {
    if (argc != 6) return 2;
    char *end; long intent = strtol(argv[5], &end, 10);
    if (*end || intent < 0 || intent > 3) return 2;
    size_t sn = 0, dn = 0, length = 0;
    unsigned char *source = read_file(argv[1], &sn), *dest = read_file(argv[2], &dn);
    float *input = read_file(argv[3], &length);
    if (!source || !dest || !input || sn < 128 || dn < 128 || length % 16
        || memcmp(source + 16, "CMYK", 4) || memcmp(dest + 16, "RGB ", 4)) return 2;
    icmErr error = {0};
    icmFile *sf = new_icmFileMem(&error, source, sn), *df = new_icmFileMem(&error, dest, dn);
    icc *sp = new_icc(&error), *dp = new_icc(&error);
    if (!sf || !df || !sp || !dp || sp->read(sp, sf, 0) || dp->read(dp, df, 0)) {
        fprintf(stderr, "Argyll profile parse failed\n"); return 3;
    }
    icRenderingIntent intents[] = {icPerceptual, icRelativeColorimetric, icSaturation, icAbsoluteColorimetric};
    icmLuSpace *forward = (icmLuSpace *)sp->get_luobj(sp, icmFwd, intents[intent], icSigXYZData, icmLuOrdNorm);
    icmLuSpace *backward = (icmLuSpace *)dp->get_luobj(dp, icmBwd, intents[intent], icSigXYZData, icmLuOrdNorm);
    if (!forward || !backward) { fprintf(stderr, "Argyll lookup construction failed\n"); return 4; }
    FILE *output = !strcmp(argv[4], "-") ? stdout : fopen(argv[4], "wb");
    if (!output) return 2;
    unsigned clipped_src = 0, clipped_dst = 0;
    for (size_t i = 0; i < length / 16; ++i) {
        double ink[4], xyz[3], rgb[3]; float result[3];
        for (unsigned c = 0; c < 4; ++c) ink[c] = input[i * 4 + c];
        int a = forward->lookup_fwd(forward, xyz, ink);
        int b = backward->lookup_fwd(backward, rgb, xyz);
        if ((a | b) & icmPe_lurv_err) { fprintf(stderr, "Argyll lookup failed at %zu\n", i); return 5; }
        clipped_src += !!(a & icmPe_lurv_clip); clipped_dst += !!(b & icmPe_lurv_clip);
        for (unsigned c = 0; c < 3; ++c) result[c] = (float)rgb[c];
        if (fwrite(result, sizeof(float), 3, output) != 3) return 2;
    }
    if (fflush(output)) return 2;
    if (output != stdout && fclose(output)) return 2;
    fprintf(stderr, "argyll pixels=%zu source_clip=%u destination_clip=%u intent=%ld bpc=off\n", length / 16, clipped_src, clipped_dst, intent);
    forward->del(forward); backward->del(backward); sp->del(sp); dp->del(dp); sf->del(sf); df->del(df);
    free(source); free(dest); free(input); return 0;
}
