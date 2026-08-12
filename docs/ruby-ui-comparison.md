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
