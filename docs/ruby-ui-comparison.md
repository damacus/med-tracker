# RubyUI comparison

Use the comparison task to inspect selected locally generated RubyUI components
against the exact version locked in `Gemfile.lock`.

```fish
task ruby-ui:compare COMPONENTS="Button Card" OUTPUT=/tmp/ruby-ui-compare
task ruby-ui:compare ALL=1 OUTPUT=/tmp/ruby-ui-compare
```

The task requires an explicit disposable destination outside the application
checkout when the checkout has uncommitted changes. It reports generated, local-only, upstream-only, and
changed files, including generated Stimulus controllers. It does not copy,
overwrite, stage, or commit any application files. Review the report and make
any intended application change separately.

The task runs the exact Bundler-activated RubyUI version locked by the app. Its
disposable copy records generator-requested gem and JavaScript dependencies
instead of applying them to the checkout; copied registration changes are also
reported. Explicit output paths must not exist and cannot traverse symbolic
links. This keeps the generated review output available after the task exits
without changing the application's dependency or runtime files.
