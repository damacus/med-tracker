#!/usr/bin/env fish

if test (count $argv) -eq 0
    echo 'Usage: with-icu.fish COMMAND [ARGS...]' >&2
    exit 2
end

set -l icu_prefix
if set -q MEDTRACKER_ICU_PREFIX
    set icu_prefix $MEDTRACKER_ICU_PREFIX
else
    switch (uname -s)
        case Darwin
            set icu_prefix (brew --prefix icu4c@78)
            or exit 1
        case Linux
            set icu_prefix /opt/medtracker-icu
        case '*'
            echo 'ICU 78.3 setup supports macOS and Linux only' >&2
            exit 1
    end
end

set -l icu_lib $icu_prefix/lib
if not test -d $icu_lib
    echo "ICU library directory is missing: $icu_lib" >&2
    exit 1
end

switch (uname -s)
    case Darwin
        set -l suffix .78.dylib
    case Linux
        set -l suffix .so.78
end

for library in icuuc icui18n icudata
    if not test -f $icu_lib/lib$library$suffix
        echo "ICU 78 library is missing: $icu_lib/lib$library$suffix" >&2
        exit 1
    end
end

set -gx RUST_ICU_MAJOR_VERSION_NUMBER 78
set -gx RUST_ICU_LINK_SEARCH_DIR $icu_lib
if test (uname -s) = Linux
    if test -n "$LD_LIBRARY_PATH"
        set -gx LD_LIBRARY_PATH "$icu_lib:$LD_LIBRARY_PATH"
    else
        set -gx LD_LIBRARY_PATH $icu_lib
    end
end

exec $argv
