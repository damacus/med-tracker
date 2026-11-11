function __stash_drop_options
    set -l command (commandline -opc)[1]
    $command --help | string match -ra -- '--[a-z][a-z-]*|-h' | sort -u
end

complete -c stash_drop -f -a '(__stash_drop_options)'
