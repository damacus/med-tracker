#include <stdio.h>
#include <unicode/ucol.h>
#include <unicode/uversion.h>

#if U_ICU_VERSION_MAJOR_NUM != 78 || U_ICU_VERSION_MINOR_NUM != 3
#error ICU 78.3 headers are required
#endif

int main(void) {
    UVersionInfo version;
    u_getVersion(version);
    if (version[0] != 78 || version[1] != 3) {
        fprintf(stderr, "Unexpected ICU runtime version: %u.%u\n", version[0], version[1]);
        return 1;
    }

    UErrorCode status = U_ZERO_ERROR;
    UCollator *collator = ucol_open("en_US", &status);
    if (U_FAILURE(status) || collator == NULL) {
        fprintf(stderr, "ICU collation data failed: %s\n", u_errorName(status));
        return 1;
    }
    UCollationResult result = ucol_strcollUTF8(collator, "a", 1, "b", 1, &status);
    ucol_close(collator);
    if (U_FAILURE(status) || result != UCOL_LESS) {
        fprintf(stderr, "ICU collation operation failed: %s\n", u_errorName(status));
        return 1;
    }
    puts("ICU 78.3 collation runtime verified");
    return 0;
}
