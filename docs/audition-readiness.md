# Audition readiness baseline

This is a static Ractor-readiness inventory, not a change request. The findings below were recorded without applying Audition fixes.

## Evidence run

The disposable scanner used Audition `0.2.1` on Ruby `4.0.6`:

```fish
set -lx AUDITION_HOME /tmp/med-tracker-audition.sj9pQy
env GEM_HOME=$AUDITION_HOME GEM_PATH=$AUDITION_HOME gtimeout --foreground 90s $AUDITION_HOME/bin/audition app --static-only --no-baseline --plain
env GEM_HOME=$AUDITION_HOME GEM_PATH=$AUDITION_HOME gtimeout --foreground 90s $AUDITION_HOME/bin/audition lib --static-only --no-baseline --plain
env GEM_HOME=$AUDITION_HOME GEM_PATH=$AUDITION_HOME gtimeout --foreground 90s $AUDITION_HOME/bin/audition config --static-only --no-baseline --plain
env GEM_HOME=$AUDITION_HOME GEM_PATH=$AUDITION_HOME gtimeout --foreground 90s $AUDITION_HOME/bin/audition scripts --static-only --no-baseline --plain
```

| Target | Result | Categories |
| --- | ---: | --- |
| `app` | 20 errors | 17 `mutable-constants`; 3 `class-level-state` |
| `lib` | 3 errors | 3 `mutable-constants` |
| `config` | 1 warning, 2 info | `Pagy::OPTIONS` mutation; two OpenTelemetry `ENV` mutations |
| `scripts` | 3 errors | `$PROGRAM_NAME` reads, `global-variables` |

The production-target errors are therefore 20 mutable-constant findings and 3 class-level-state findings. The scripts findings are main-only guards in `scripts/audit_oidc_security.rb:128`, `scripts/build_curated_product_yaml.rb:134`, and `scripts/search_grocery_vitamins.rb:220`; they are recorded separately and are not production Ractor paths.

## Application-owned paths

- App mutable constants: `app/components/medications/dosage_options_fields.rb:6`, `app/components/medications/wizard/step_indicator.rb:7`, `app/components/shared/person_avatar.rb:8`, `app/controllers/concerns/medication_form_context.rb:6`, and `app/controllers/pwa_controller.rb:31`.
- More app mutable constants: `app/models/medication_dosage.rb:57`, `app/services/audit/verification/database_authority.rb:14`, `app/services/care_delegation/assign.rb:10`, `app/services/medication_onboarding_create_service.rb:12`, `app/services/medication_review_source_instruction_classifier.rb:5`, and `app/services/nhs_dmd/dosage_form_filter.rb:23`.
- Remaining app mutable constants: `app/services/paid_feature.rb:8`, `app/services/reports/medication_review_pdf/opening_sections.rb:6`, `app/views/profiles/experiments_card.rb:8` and `:25`, and `app/views/profiles/theme_picker_card.rb:6` and `:12`.
- Class-level state: `app/misc/rodauth_main.rb:539` and `:764`, plus `app/services/barcode_catalog/curated_products.rb:93`.
- Library mutable constants: `lib/hosted_restore/rehearsal.rb:410` and `:432`, and `lib/otel/database_connection_pool_metrics.rb:5`.
- Configuration: `config/initializers/pagy.rb:4` is the warning; `config/initializers/opentelemetry.rb:121` and `:122` are informational `ENV` mutations.

## Full-root limitation and next step

An earlier direct host full-root scan exceeded 90 seconds. The bounded host command used for this evidence run, `audition . --static-only --no-baseline --plain`, completed quickly but reported only the Rails application, library, and configuration paths; it did not include the separate scripts inventory. Host scan behavior must therefore not be used to create the repository baseline.

Once Docker is available, run `task audition:static`, review the application-owned findings, then run `task audition:baseline` and review the generated `.audition-baseline.json` before committing it. That Docker/CI-generated baseline is the only reviewed incremental baseline. `task audition:ci` remains the future CI gate; dependency and dynamic scans remain report-only until Task 7 wires CI.
