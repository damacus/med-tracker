#!/usr/bin/env fish

set -l release_apk phone/build/outputs/apk/release/phone-release-unsigned.apk

if not test -f $release_apk
    echo "Release APK is missing: $release_apk" >&2
    exit 1
end

set -l dex_entries (unzip -Z1 $release_apk | string match 'classes*.dex')

if test (count $dex_entries) -eq 0
    echo "Release APK contains no classes*.dex entries" >&2
    exit 1
end

for forbidden in \
    'Lio/medtracker/client/models/AuthLoginRequest;' \
    createLoginSession \
    /auth/login
    for dex_entry in $dex_entries
        if unzip -p $release_apk $dex_entry | strings | string match --quiet "*$forbidden*"
            echo "Release APK contains password authentication transport: $forbidden" >&2
            exit 1
        end
    end
end
