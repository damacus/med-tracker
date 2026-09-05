set -l forbidden '(https?://(?!schemas.android.com/)|access.?token|refresh.?token|password|server.?url|base.?url|io[./]medtracker[./]client|okhttp|retrofit|ktor|java.net|javax.net|android.webkit|android.permission.INTERNET|MessageClient|SharedPreferences|android.security|java.io.File|SQLite)'

if rg --pcre2 -ni "$forbidden" wear/src wear-protocol/src/main --glob '!**/test/**'
    echo 'Wear source contains a forbidden credential, Rails client, storage or direct transport interface.' >&2
    exit 1
else if test $status -ne 1
    echo 'Wear source boundary inspection failed.' >&2
    exit 1
end

set -l sdk_root "$ANDROID_HOME"
if test -z "$sdk_root"
    set sdk_root "$ANDROID_SDK_ROOT"
end
set -l aapt (find "$sdk_root/build-tools" -name aapt -type f | sort | tail -1)
set -l apksigner (find "$sdk_root/build-tools" -name apksigner -type f | sort | tail -1)
if test -z "$aapt"; or test -z "$apksigner"
    echo 'Android SDK aapt and apksigner are required for Wear APK boundary inspection.' >&2
    exit 1
end

for variant in debug staging release
    set -l apk wear/build/outputs/apk/$variant/wear-$variant.apk
    if test $variant = release
        set apk wear/build/outputs/apk/release/wear-release-unsigned.apk
    end
    if not test -f "$apk"
        echo "Wear APK is missing: $apk. Run task android:assemble first." >&2
        exit 1
    end
    set -l phone_apk phone/build/outputs/apk/$variant/phone-$variant.apk
    if test $variant = release
        set phone_apk phone/build/outputs/apk/release/phone-release-unsigned.apk
    end
    set -l phone_id ("$aapt" dump badging "$phone_apk" | string match -r "^package: name='[^']+'")
    or exit 1
    set -l wear_id ("$aapt" dump badging "$apk" | string match -r "^package: name='[^']+'")
    or exit 1
    if test "$phone_id" != "$wear_id"
        echo "Phone and Wear $variant application IDs differ." >&2
        exit 1
    end
    if test $variant != release
        set -l phone_signer ("$apksigner" verify --print-certs "$phone_apk" | string match '*certificate SHA-256 digest:*')
        or exit 1
        set -l wear_signer ("$apksigner" verify --print-certs "$apk" | string match '*certificate SHA-256 digest:*')
        or exit 1
        if test "$phone_signer" != "$wear_signer"
            echo "Phone and Wear $variant signing identities differ." >&2
            exit 1
        end
    end
    set -l permissions ("$aapt" dump permissions "$apk" | string collect)
    or exit 1
    if string match -rq 'android.permission.(INTERNET|ACCESS_NETWORK_STATE|READ_|WRITE_|GET_ACCOUNTS|USE_CREDENTIALS)' "$permissions"
        echo "Wear APK exposes a forbidden permission: $apk" >&2
        exit 1
    end
    set -l dex_entries (unzip -Z1 "$apk" | string match 'classes*.dex')
    for dex in $dex_entries
        if unzip -p "$apk" "$dex" | strings | string match -rq 'Lio/medtracker/client/|Lokhttp3/|Lretrofit2/|Lio/ktor/|https://[^ ]*medtracker'
            echo "Wear APK contains a Rails client or direct transport: $apk" >&2
            exit 1
        end
    end
end
echo 'Wear source and packaged permissions contain no Rails transport or credentials.'
