if not set -q PHONE_SERIAL; or not set -q WATCH_SERIAL; or test -z "$PHONE_SERIAL"; or test -z "$WATCH_SERIAL"
    echo 'Set PHONE_SERIAL and WATCH_SERIAL to an already-paired phone and Wear emulator. No devices were changed.' >&2
    exit 1
end
if test "$PHONE_SERIAL" = "$WATCH_SERIAL"
    echo 'Phone and watch serials must be distinct.' >&2
    exit 1
end
for serial in "$PHONE_SERIAL" "$WATCH_SERIAL"
    if not string match -rq '^emulator-[0-9]+$' "$serial"
        echo 'Only explicit emulator serials are allowed; physical devices are never used.' >&2
        exit 1
    end
end
set -l sdk_root "$ANDROID_HOME"
if test -z "$sdk_root"
    set sdk_root "$ANDROID_SDK_ROOT"
end
set -l adb "$sdk_root/platform-tools/adb"
if not test -x "$adb"
    echo 'Set ANDROID_HOME to an existing Android SDK with adb.' >&2
    exit 1
end
for serial in "$PHONE_SERIAL" "$WATCH_SERIAL"
    set -l device_state ("$adb" -s "$serial" get-state 2>/dev/null)
    if test "$device_state" != device
        echo "Selected emulator is unavailable: $serial. Start and pair both emulators first." >&2
        exit 1
    end
    set -l emulator_state ("$adb" -s "$serial" shell getprop ro.kernel.qemu | string trim)
    if test "$emulator_state" != 1
        echo "Selected device is not an emulator: $serial" >&2
        exit 1
    end
    if not "$adb" -s "$serial" shell pm path com.google.android.gms | string match -q 'package:*'
        echo "Google Play services is missing on $serial" >&2
        exit 1
    end
end
if not "$adb" -s "$WATCH_SERIAL" shell getprop ro.build.characteristics | string match -q '*watch*'
    echo 'WATCH_SERIAL must identify a Wear emulator.' >&2
    exit 1
end
if "$adb" -s "$PHONE_SERIAL" shell getprop ro.build.characteristics | string match -q '*watch*'
    echo 'PHONE_SERIAL must identify a phone emulator.' >&2
    exit 1
end
set -l package io.damacus.medtracker.debug
if "$adb" -s "$PHONE_SERIAL" shell pm path "$package" | string match -q 'package:*'
    echo 'The phone debug app is already installed. Use a clean test emulator; this smoke will not erase an existing session.' >&2
    exit 1
end

task assemble
or exit 1
set -l phone_apk phone/build/outputs/apk/debug/phone-debug.apk
set -l watch_apk wear/build/outputs/apk/debug/wear-debug.apk
set -l apksigner (find "$sdk_root/build-tools" -name apksigner -type f | sort | tail -1)
set -l aapt (find "$sdk_root/build-tools" -name aapt -type f | sort | tail -1)
if test -z "$apksigner"; or test -z "$aapt"
    echo 'SDK apksigner and aapt are required.' >&2
    exit 1
end
set -l phone_signer ("$apksigner" verify --print-certs "$phone_apk" | string match '*certificate SHA-256 digest:*')
or exit 1
set -l watch_signer ("$apksigner" verify --print-certs "$watch_apk" | string match '*certificate SHA-256 digest:*')
or exit 1
if test "$phone_signer" != "$watch_signer"
    echo 'Phone and Wear debug signing identities do not match.' >&2
    exit 1
end
for apk in "$phone_apk" "$watch_apk"
    if not "$aapt" dump badging "$apk" | string match -q "package: name='$package' *"
        echo 'Phone and Wear application IDs do not match.' >&2
        exit 1
    end
end
"$adb" -s "$PHONE_SERIAL" install "$phone_apk"
or exit 1
"$adb" -s "$WATCH_SERIAL" install -r "$watch_apk"
or exit 1
"$adb" -s "$WATCH_SERIAL" shell am force-stop "$package"
or exit 1
"$adb" -s "$PHONE_SERIAL" shell am start -W -n "$package/io.damacus.medtracker.MainActivity"
or exit 1
sleep 10
"$adb" -s "$WATCH_SERIAL" shell am start -W -n "$package/io.damacus.medtracker.wear.MainActivity"
or exit 1
function wait_for_signed_out -a adb serial
    for attempt in (seq 1 30)
        "$adb" -s "$serial" shell uiautomator dump /data/local/tmp/medtracker-wear-smoke.xml >/dev/null
        if "$adb" -s "$serial" shell cat /data/local/tmp/medtracker-wear-smoke.xml | string match -q '*Phone connected. Sign in on your phone.*'
            return 0
        end
        sleep 2
    end
    echo 'Wear did not converge to signed-out status. Verify the selected emulator pair is paired in Android Studio.' >&2
    return 1
end

wait_for_signed_out "$adb" "$WATCH_SERIAL"
or exit 1
"$adb" -s "$WATCH_SERIAL" shell am force-stop "$package"
or exit 1
"$adb" -s "$WATCH_SERIAL" shell am start -W -n "$package/io.damacus.medtracker.wear.MainActivity"
or exit 1
wait_for_signed_out "$adb" "$WATCH_SERIAL"
or exit 1
echo 'PASS: Wear loaded signed-out status after phone publication and reloaded the persistent status after a watch process restart.'
