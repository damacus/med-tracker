#!/usr/bin/env fish
set -l phase node
for argument in $argv
    if test "$argument" = PHASE=prepare
        set phase prepare
    else if test "$argument" = PHASE=restore
        set phase restore
    end
end
if not contains -- "CONTRACT_PROJECT=$MINOR_WRAPPER_EXPECT_PROJECT" $argv
    exit 90
end
if test "$phase" = node
    if test "$argv[1]" != api:contract-browser-node; or not contains -- "BROWSER_TEST_FILES=$MINOR_WRAPPER_EXPECT_FILES" $argv
        exit 91
    end
else
    if test "$argv[1]" != api:contract-minor-viewer-sql; or not contains -- "FIXTURE_PATH=$MINOR_WRAPPER_EXPECT_FIXTURE" $argv
        exit 92
    end
end
printf '%s\n' "$phase" >>"$MINOR_WRAPPER_RECEIPT"
switch "$phase"
    case prepare
        exit $MINOR_WRAPPER_PREPARE_EXIT
    case restore
        exit $MINOR_WRAPPER_RESTORE_EXIT
    case node
        exit $MINOR_WRAPPER_BROWSER_EXIT
end

