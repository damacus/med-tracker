#!/usr/bin/env fish

set -l repo_root (git rev-parse --show-toplevel 2>/dev/null)
if test -z "$repo_root"
    echo "translator hook: not inside a git repository"
    exit 1
end

cd $repo_root

set -l staged_files (git diff --cached --name-only --diff-filter=ACMR)
set -l relevant_files

for file in $staged_files
    if string match -rq '^config/locales/.*\.yml$' -- $file
        set relevant_files $relevant_files $file
        continue
    end

    if string match -rq '^app/components/ruby_ui/typography/.*\.rb$' -- $file
        set relevant_files $relevant_files $file
    end
end

if test (count $relevant_files) -eq 0
    echo "translator hook: no locale or typography changes detected"
    exit 0
end

set -l model gpt-5.4-mini
if set -q CODEX_TRANSLATOR_MODEL
    set model $CODEX_TRANSLATOR_MODEL
end

set -l prompt (string join \n \
    "Use the translator skill from ~/.agents/skills/translator." \
    "Repository: MedTracker." \
    "Changed files: "(string join ", " $relevant_files) \
    "Update translation files under config/locales so all locale trees stay in sync." \
    "When typography components introduce user-facing copy, add the corresponding i18n keys and translations." \
    "Preserve existing translations when still accurate. Preserve interpolation tokens, pluralization keys, and YAML structure." \
    "Only modify files that are necessary for translation maintenance, primarily config/locales/*.yml." \
    "Do not make unrelated refactors. Finish after writing the required locale updates.")

echo "translator hook: running Codex translator with model $model"

codex exec \
    --model $model \
    --cd $repo_root \
    --add-dir /Users/damacus/.agents/skills \
    --sandbox workspace-write \
    --full-auto \
    $prompt

if test $status -ne 0
    echo "translator hook: Codex run failed"
    exit 1
end

set -l locale_files (find config/locales -maxdepth 1 -type f | sort)
if test (count $locale_files) -gt 0
    git add $locale_files
end

echo "translator hook: staged updated locale files"
