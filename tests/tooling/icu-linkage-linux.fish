#!/usr/bin/env fish

set -l root (path resolve (dirname (status filename))/../..)
set -l fixture (mktemp -d /tmp/medtracker-icu-linkage.XXXXXX)
mkdir -p $fixture/bin $fixture/build/debug/deps $fixture/target/debug/deps
echo '#!/usr/bin/env fish
echo Linux' >$fixture/bin/uname
echo '#!/usr/bin/env fish
echo $argv >>$MEDTRACKER_ICU_FIXTURE/cc-args
set -l output (contains -i -- -o $argv)
cp $MEDTRACKER_ICU_FIXTURE/probe $argv[(math $output + 1)]
chmod +x $argv[(math $output + 1)]' >$fixture/bin/cc
echo '#!/usr/bin/env fish
if test "$MEDTRACKER_ICU_CASE" = runtime-version
    echo "Unexpected ICU runtime version: 77.1" >&2
    exit 1
end
if test "$MEDTRACKER_ICU_CASE" = runtime-data
    echo "ICU collation data failed" >&2
    exit 1
end
touch $MEDTRACKER_ICU_FIXTURE/executed
echo "ICU 78.3 collation runtime verified"' >$fixture/probe
echo '#!/usr/bin/env fish
echo "{\"target_directory\":\"$MEDTRACKER_ICU_FIXTURE/target\",\"build_directory\":\"$MEDTRACKER_ICU_FIXTURE/build\"}"' >$fixture/bin/cargo
echo '#!/usr/bin/env fish
if string match -q "*application*" $argv[-1]
    touch $MEDTRACKER_ICU_FIXTURE/application-inspected
    if test "$MEDTRACKER_ICU_CASE" = application-wrong
        echo "libicuuc.so.78 => /wrong/libicuuc.so.78 (0x1)"
        exit 0
    end
    if test "$MEDTRACKER_ICU_CASE" = application-partial
        echo "libicuuc.so.78 => /opt/medtracker-icu/lib/libicuuc.so.78 (0x1)"
        echo "libicudata.so.78 => /opt/medtracker-icu/lib/libicudata.so.78 (0x1)"
        exit 0
    end
    echo "libc.so.6 => /lib/libc.so.6 (0x1)"
    exit 0
end
for library in icuuc icui18n icudata
    if test "$MEDTRACKER_ICU_CASE" = missing; and test $library = icudata
        echo "lib$library.so.78 => not found"
    else if test "$MEDTRACKER_ICU_CASE" = wrong; and test $library = icui18n
        echo "lib$library.so.78 => /wrong/lib$library.so.78 (0x1)"
    else
        echo "lib$library.so.78 => /opt/medtracker-icu/lib/lib$library.so.78 (0x1)"
    end
end' >$fixture/bin/ldd
touch $fixture/build/debug/deps/application
chmod +x $fixture/bin/* $fixture/probe $fixture/build/debug/deps/application
set -l fixture_path (string join ':' $fixture/bin $PATH)

for scenario in success application-partial missing wrong runtime-version runtime-data application-wrong
    rm -f $fixture/executed $fixture/application-inspected
    env PATH=$fixture_path RUST_ICU_LINK_SEARCH_DIR=/opt/medtracker-icu/lib MEDTRACKER_ICU_FIXTURE=$fixture MEDTRACKER_ICU_CASE=$scenario bash $root/scripts/check-icu-linkage-linux.sh >$fixture/$scenario.log 2>&1
    set -l result $status
    if contains -- $scenario success application-partial
        if test $result -ne 0; or not test -e $fixture/executed; or not test -e $fixture/application-inspected
            cat $fixture/$scenario.log >&2
            echo 'Runtime probe or Cargo build-directory inspection did not run' >&2
            exit 1
        end
    else if test $result -eq 0
        echo "ICU runtime gate accepted $scenario" >&2
        exit 1
    end
end
if not string match -q -- '*-Wl,--no-as-needed*' (cat $fixture/cc-args)
    echo 'Runtime probe does not force retention of all three ICU libraries' >&2
    exit 1
end
rm -rf -- $fixture
echo 'Linux ICU runtime gate fixtures passed'
