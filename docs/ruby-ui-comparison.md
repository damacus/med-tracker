# RubyUI comparison

Use the comparison task to inspect selected locally generated RubyUI components
against the exact version locked in `Gemfile.lock`.

```fish
task ruby-ui:compare COMPONENTS="Button Dialog"
task ruby-ui:compare COMPONENTS="Button Dialog" OUTPUT=/private/tmp/ruby-ui-compare
```

The task accepts named component families only. It runs the locked RubyUI
generator in a fresh external copy with dependency installation disabled, then
reports unchanged, changed, missing locally, and local-only Ruby and JavaScript
files for those families. Dependency declarations from RubyUI's own metadata
are shown for information only.

An explicit output path keeps the generated files for inspection. It must be a
new path outside the checkout and cannot be or traverse a symbolic link. The
task never changes the checkout, application dependencies, Importmap, or local
RubyUI runtime files.
