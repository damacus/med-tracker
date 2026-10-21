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

set -l fixture_root (mktemp -d /tmp/medtracker-icu-test.XXXXXX)
set -l fake_bin $fixture_root/bin
set -l fake_prefix $fixture_root/icu
set -l fake_lib $fake_prefix/lib
mkdir -p $fake_bin $fake_lib
echo '#!/usr/bin/env fish
echo Linux' > $fake_bin/uname
chmod +x $fake_bin/uname
for library in icuuc icui18n icudata
    touch $fake_lib/lib$library.so.78
end

set -l fake_path (string join ':' $fake_bin $PATH)
set -l unset_path (env -u LD_LIBRARY_PATH PATH=$fake_path MEDTRACKER_ICU_PREFIX=$fake_prefix fish $wrapper printenv LD_LIBRARY_PATH)
set -l empty_path (env LD_LIBRARY_PATH= PATH=$fake_path MEDTRACKER_ICU_PREFIX=$fake_prefix fish $wrapper printenv LD_LIBRARY_PATH)
set -l existing_path (env LD_LIBRARY_PATH=/previous/lib PATH=$fake_path MEDTRACKER_ICU_PREFIX=$fake_prefix fish $wrapper printenv LD_LIBRARY_PATH)
rm -rf -- $fixture_root

if test "$unset_path" != "$fake_lib"
    echo "Unset Linux LD_LIBRARY_PATH produced '$unset_path'" >&2
    exit 1
end
if test "$empty_path" != "$fake_lib"
    echo "Empty Linux LD_LIBRARY_PATH produced '$empty_path'" >&2
    exit 1
end
if test "$existing_path" != "$fake_lib:/previous/lib"
    echo "Existing Linux LD_LIBRARY_PATH produced '$existing_path'" >&2
    exit 1
end

exit 0
