#!/usr/bin/env fish

set -l root (path resolve (dirname (status filename))/../..)
set -l wrapper $root/scripts/with-icu.fish

set -l major (fish $wrapper printenv RUST_ICU_MAJOR_VERSION_NUMBER)
or exit 1
test "$major" = 78
or exit 1
set -l library_dir (fish $wrapper printenv RUST_ICU_LINK_SEARCH_DIR)
or exit 1
test -d "$library_dir"
or exit 1

fish $wrapper true
or exit 1

fish $wrapper false
and exit 1

MEDTRACKER_ICU_PREFIX=/nonexistent/medtracker-icu fish $wrapper true 2>/dev/null
and exit 1

exit 0
