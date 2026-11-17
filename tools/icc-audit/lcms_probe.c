/* ICC deviation witnesses. Build against an explicitly chosen lcms source. */
#include "lcms2.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("lcms encoded version=%d\n", cmsGetEncodedCMMversion());
    const int sizes[] = {4096, 65530}; /* lcms's maximum is 65530 */
    for (int size = 0; size < 2; size++) {
        int n = sizes[size];
        for (int kind = 0; kind < 3; kind++) {
            double plateau = kind == 0 ? 0.5 : kind == 1 ? 0.8 : 1.0;
            cmsUInt16Number *table = malloc((size_t)n * sizeof(*table));
            if (!table) return 1;
            for (int i = 0; i < n; i++) {
                double x = (double)i / (n - 1), y;
                if (kind == 0)
                    y = x < 0.3 ? x / 0.3 * plateau : x <= 0.6 ? plateau :
                        plateau + (x - 0.6) / 0.4 * (1.0 - plateau);
                else y = x < 0.5 ? x / 0.5 * plateau : plateau;
                table[i] = (cmsUInt16Number)floor(y * 65535.0 + 0.5);
            }
            cmsToneCurve *fwd = cmsBuildTabulatedToneCurve16(NULL, n, table);
            if (!fwd) return 2;
            cmsToneCurve *inv = cmsReverseToneCurveEx(n, fwd);
            if (!inv) return 2;
            float stored = (float)floor(plateau * 65535.0 + 0.5) / 65535.0f;
            printf("D14 n=%d %s: decimal=%.9f, stored=%.9f\n", n,
                   kind == 0 ? "mid" : kind == 1 ? "end" : "end_one",
                   cmsEvalToneCurveFloat(inv, (float)plateau),
                   cmsEvalToneCurveFloat(inv, stored));
            cmsFreeToneCurve(inv);
            cmsFreeToneCurve(fwd);
            free(table);
        }
    }
    double params[3] = {1.0, 2.0, 0.0};
    cmsToneCurve *curve = cmsBuildParametricToneCurve(NULL, 2, params);
    if (!curve) return 3;
    printf("D13 x=0.75 -> %.9f\n", cmsEvalToneCurveFloat(curve, 0.75f));
    cmsFreeToneCurve(curve);
    cmsToneCurve *gamma = cmsBuildGamma(NULL, 2.2);
    cmsCIExyY d65 = {0.3127, 0.3290, 1.0};
    cmsHPROFILE gray = cmsCreateGrayProfile(&d65, gamma);
    if (!gray) return 4;
    const cmsCIEXYZ *wp = cmsReadTag(gray, cmsSigMediaWhitePointTag);
    if (!wp) return 5;
    printf("D3 gray wtpt=[%.9f, %.9f, %.9f]; chad=%d\n",
           wp->X, wp->Y, wp->Z, cmsIsTag(gray, cmsSigChromaticAdaptationTag));
    cmsCloseProfile(gray);
    cmsFreeToneCurve(gamma);
    for (int size = 12; size <= 44; size += 32) {
        unsigned char tag[44] = {'c', 'i', 'c', 'p', 0, 0, 0, 0, 2, 13, 0, 1};
        const double xy[] = {0.64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290};
        for (int i = 0; i < 8; i++) {
            unsigned v = (unsigned)floor(xy[i] * 65536.0 + 0.5);
            for (int j = 0; j < 4; j++) tag[12 + 4*i + j] = (unsigned char)(v >> (24 - 8*j));
        }
        cmsHPROFILE profile = cmsCreate_sRGBProfile();
        cmsUInt32Number length = 0;
        if (!profile || !cmsWriteRawTag(profile, cmsSigcicpTag, tag, size) ||
            !cmsSaveProfileToMem(profile, NULL, &length)) return 6;
        void *bytes = malloc(length);
        if (!bytes || !cmsSaveProfileToMem(profile, bytes, &length)) return 7;
        cmsCloseProfile(profile);
        profile = cmsOpenProfileFromMem(bytes, length);
        if (!profile) return 8;
        int accepted = cmsReadTag(profile, cmsSigcicpTag) != NULL;
        printf("D15 cicp bytes=%d, parsed=%d\n", size, accepted);
        cmsCloseProfile(profile);
        free(bytes);
        if (accepted != (size == 12)) return 9;
    }
    return 0;
}
