#!/usr/bin/env fish

set -l arguments
for component in (string split ' ' -- $RUBY_UI_COMPONENTS)
    test -n "$component"; and set -a arguments $component
end
if test -n "$RUBY_UI_OUTPUT"
    set -p arguments --output "$RUBY_UI_OUTPUT"
end

mise exec -- bundle exec ruby scripts/compare_ruby_ui.rb $arguments
