#!/usr/bin/env fish

set -l script_dir (dirname (status filename))

if not "$script_dir/generate-swift.fish"
    exit 1
end

if not "$script_dir/generate-kotlin.fish"
    exit 1
end
