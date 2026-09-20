/* Native libc preflight: fail rather than silently test the C locale. */
#include <errno.h>
#include <langinfo.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <wctype.h>

int main(void)
{
    const char *name = setlocale(LC_ALL, "ja_JP.SJIS");
    if (!name) {
        fputs("ja_JP.SJIS is unavailable\n", stderr);
        return 1;
    }
    const char *codeset = nl_langinfo(CODESET);
    printf("locale=%s codeset=%s MB_CUR_MAX=%zu wchar_t=%zu\n",
           name, codeset, (size_t)MB_CUR_MAX, sizeof(wchar_t));
    if ((strcmp(codeset, "SHIFT_JIS") && strcmp(codeset, "SJIS")) || MB_CUR_MAX != 2)
        return 1;
    mbstate_t state = {0};
    wchar_t wc = 0;
    size_t n = mbrtowc(&wc, "\x82\x81", 2, &state);
    printf("decode=%zu wchar=0x%04lx\n", n, (unsigned long)wc);
    if (n != 2)
        return 1;
    char bytes[16];
    memset(&state, 0, sizeof state);
    n = wcrtomb(bytes, towupper(wc), &state);
    printf("encode=%zu\n", n);
    if (n != 2 || memcmp(bytes, "\x82\x60", 2))
        return 1;
    memset(&state, 0, sizeof state);
    n = mbrtowc(&wc, "\xe9", 1, &state);
    printf("truncated=%s\n", n == (size_t)-2 ? "incomplete" : "unexpected");
    if (n != (size_t)-2)
        return 1;
    memset(&state, 0, sizeof state);
    errno = 0;
    n = mbrtowc(&wc, "\x82\x20", 2, &state);
    printf("invalid=%s errno=%d\n", n == (size_t)-1 ? "error" : "unexpected", errno);
    return !(n == (size_t)-1 && errno == EILSEQ);
}
