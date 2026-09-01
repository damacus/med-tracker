# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Taskfiles' do
  it 'defines opt-in Portless tasks for dev and test' do
    expect(dev_taskfile.dig('tasks', 'portless', 'cmds')).to include('./scripts/portless_oidc.fish dev med-tracker')
    expect(test_taskfile.dig('tasks', 'portless', 'cmds')).to include(
      './scripts/portless_oidc.fish test med-tracker-test'
    )
  end

  it 'requires a globally installed Portless CLI' do
    expect(portless_script).to include('type -q portless')
    expect(portless_script).to include('npm install -g portless')
    expect(portless_script).not_to include('npx portless')
  end

  it 'probes the clean HTTPS URL after registering the alias' do
    expect(portless_script).to include(
      'set portless_ready false',
      'for attempt in (seq 1 10)',
      'curl --silent --show-error --head --fail --max-time 5 $base_url',
      'sleep 1',
      'Portless did not respond at $base_url.',
      'portless proxy start --port 443 --https --force'
    )
  end

  it 'uses test-scoped OIDC variables for the Portless test task' do
    expect(portless_script).to include('set -lx TEST_APP_URL $base_url')
    expect(portless_script).to include('set -lx TEST_OIDC_CLIENT_ID $OIDC_CLIENT_ID')
    expect(portless_script).to include('set -lx TEST_OIDC_REDIRECT_URI $callback_url')
  end

  it 'uses a worktree-specific Docker Compose project name' do
    compose_project = internal_taskfile.dig('vars', 'COMPOSE_PROJECT', 'sh')

    expect(compose_project).to include('pwd -P')
    expect(compose_project).to include('git hash-object --stdin')
  end

  it 'defines a Docker test preflight with distinct failure messages' do
    commands = test_taskfile.dig('tasks', 'preflight', 'cmds')

    expect(commands).to include(
      './scripts/test_preflight.fish {{ .TEST_FILE | default "spec/config/taskfiles_spec.rb" }}'
    )
    expect(test_preflight_script).to include(
      'Docker is unavailable',
      'Test image med-tracker-web-test is missing',
      'Test preflight spec failed'
    )
  end

  it 'runs RuboCop through the repository binstub' do
    command = root_taskfile.dig('tasks', 'rubocop', 'cmds', 0, 'vars', 'COMMAND')

    expect(command).to start_with('bin/rubocop --force-exclusion')
  end

  it 'defines an advisory DoseConstraints typecheck task' do
    task = root_taskfile.dig('tasks', 'typecheck')

    expect(task.dig('cmds', 0, 'task')).to eq('internal:run')
    expect(task.dig('cmds', 0, 'vars', 'ENVIRONMENT')).to eq('test')
    expect(task.dig('cmds', 0, 'vars', 'SERVICE')).to eq('tools-test')
    expect(task.dig('cmds', 0, 'vars', 'COMMAND')).to eq(
      'bundle exec srb tc --ignore /usr/local/bundle/gems/prism app/models/dose_constraints.rb ' \
      'app/domain/dose_cycle.rb sorbet/rbi/dose_constraints.rbi'
    )
  end

  it 'defines deterministic native API client generation tasks' do
    expect(api_client_generation_task_commands).to eq(
      'api-clients:generate' => ['./client-tools/openapi-generator/generate.fish'],
      'api-clients:verify-generated' => ['./client-tools/openapi-generator/generate.fish determinism'],
      'api-clients:kotlin' => ['./client-tools/openapi-generator/generate-kotlin.fish'],
      'api-clients:swift' => ['./client-tools/openapi-generator/generate-swift.fish'],
      'api-clients:verify' => ['api-clients:verify-generated']
    )
  end

  it 'generates native clients before compiling and testing them' do
    expect(api_client_test_task_commands).to eq(
      'api-clients:kotlin:test' => [
        'api-clients:kotlin',
        './tmp/api-clients/kotlin/gradlew --no-daemon -p tmp/api-clients/kotlin check assemble'
      ],
      'api-clients:swift:test' => [
        'api-clients:swift',
        'swift build --package-path tmp/api-clients/swift'
      ]
    )
  end

  it 'keeps generated native clients disposable and omits generated reference docs' do
    expect(swift_generator_script).to include(
      'set -l output "$repo_root/tmp/api-clients/swift"',
      '--global-property apiDocs=false,modelDocs=false'
    )
    expect(kotlin_generator_script).to include(
      'set -l output "$repo_root/tmp/api-clients/kotlin"',
      '--global-property apiDocs=false,modelDocs=false'
    )
    expect(Rails.root.join('client-tools/generated')).not_to exist
  end

  it 'generates native clients once and passes ephemeral output to compile jobs' do
    jobs = api_client_workflow.fetch('jobs')
    generation_steps = job_step_names(jobs.fetch('api_client_generation'))

    expect(generation_steps).to include('Set up Task', 'Install Fish', 'Upload generated clients')
    expect(job_step_names(jobs.fetch('api_client_kotlin'))).to include('Download generated clients')
    expect(job_step_names(jobs.fetch('api_client_swift'))).to include('Download generated clients')
    expect(api_client_workflow_source).not_to include('apt-get update')
    expect(api_client_workflow_source).to include(
      'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a',
      'actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c'
    )
  end

  it 'pins the native API generator image and language generators' do
    expect(swift_generator_script).to include(
      generator_image,
      '-g swift6'
    )
    expect(kotlin_generator_script).to include(
      generator_image,
      '-g kotlin'
    )
  end

  it 'keeps native API generator entry points executable' do
    expect(
      [
        Rails.root.join('client-tools/openapi-generator/generate.fish'),
        Rails.root.join('client-tools/openapi-generator/generate-swift.fish'),
        Rails.root.join('client-tools/openapi-generator/generate-kotlin.fish')
      ]
    ).to all(be_executable)
  end

  it 'runs generator containers as the invoking host user' do
    expect(swift_generator_script).to include('docker run --rm --user "$host_uid:$host_gid"')
    expect(kotlin_generator_script).to include('docker run --rm --user "$host_uid:$host_gid"')
  end

  it 'serializes Docker Compose runs within each worktree environment' do
    command = internal_taskfile.dig('tasks', 'run', 'cmds', 0)

    expect(command).to include('./scripts/with_compose_lock.rb "{{ .COMPOSE_PROJECT }}-{{ .ENVIRONMENT }}"')
    expect(compose_lock_script).to include(
      'lock.flock(File::LOCK_EX)',
      'Process.spawn(*command)',
      'Process.wait2(pid)'
    )
  end

  it 'runs database migrations through the migration service' do
    migration_task = internal_taskfile.dig('tasks', 'db-migrate')

    expect(migration_task.dig('vars', 'MIGRATION_SERVICE')).to eq('migrate-{{ .ENVIRONMENT }}')
    expect(migration_task.dig('cmds', 0)).to include('{{ .MIGRATION_SERVICE }} rails db:migrate')
    expect(migration_task.to_json).not_to include('WEB_SERVICE')
  end

  it 'defines a Vernier dashboard profiling task' do
    task = root_taskfile.dig('tasks', 'profile:dashboard')
    commands = task.fetch('cmds')

    expect(gemfile).to include("gem 'vernier', '~> 1.10', require: false")
    expect(commands.dig(0, 'task')).to eq('internal:run')
    expect(commands.dig(0, 'vars', 'ENVIRONMENT')).to eq('dev')
    expect(commands.dig(0, 'vars', 'COMMAND')).to include(
      'bundle exec vernier run',
      '--output {{ .output }}',
      '--hooks rails,memory_usage',
      'bin/rails runner scripts/profile_dashboard_request.rb'
    )
  end

  it 'profiles a representative dashboard request pipeline' do
    expect(dashboard_profile_script).to include(
      'Account.find_by!(email: profile_email)',
      'DashboardPresenter.new(',
      'presenter.routine_tasks_by_person',
      'presenter.as_needed_by_person',
      'presenter.today_takes_by_person',
      'File.write(summary_path, summary)'
    )
  end

  it 'runs support expiry processing with the forced-RLS application role' do
    task = root_taskfile.dig('tasks', 'support-access:expire')
    command = task.dig('cmds', 0, 'vars', 'COMMAND')

    expect(command).to eq('env DATABASE_ROLE=med_tracker_app rails support_access:expire')
  end

  it 'defines the canary demo reset operator task with the owner role' do
    task = root_taskfile.dig('tasks', 'canary:demo-reset')
    command = task.dig('cmds', 0, 'vars', 'COMMAND')

    expect(command).to eq('env DATABASE_ROLE=med_tracker_owner rails canary:demo_reset')
  end

  it 'passes export destinations as environment data instead of command text' do
    task = root_taskfile.dig('tasks', 'household-lifecycle:download') || {}
    command = task.dig('cmds', 0, 'vars', 'COMMAND')

    expect(task.dig('requires', 'vars') || []).to include('DESTINATION')
    expect(task.fetch('env', {})).to eq('DESTINATION' => '{{ .DESTINATION }}')
    expect(task.dig('cmds', 0, 'vars', 'DOCKER_RUN_ARGS')).to eq('-e DESTINATION')
    expect(command).to eq(
      'env DATABASE_ROLE=med_tracker_app HOUSEHOLD_ID={{ .HOUSEHOLD_ID }} ' \
      'ACTOR_ACCOUNT_ID={{ .ACTOR_ACCOUNT_ID }} EXPORT_ID={{ .EXPORT_ID }} rails household_lifecycle:download'
    )
    expect(command).not_to include('DESTINATION')
  end

  it 'passes RubyUI comparison input as environment data instead of shell command text' do
    task = root_taskfile.dig('tasks', 'ruby-ui:compare')
    command = task.dig('cmds', 0)

    expect(task.fetch('env')).to eq(
      'RUBY_UI_COMPONENTS' => '{{ .COMPONENTS | default "" }}',
      'RUBY_UI_OUTPUT' => '{{ .OUTPUT | default "" }}'
    )
    expect(command).to eq('./scripts/run_ruby_ui_comparison.fish')
    expect(command).not_to include('.OUTPUT', '.COMPONENTS')
  end

  it 'uses an executable Fish wrapper without evaluating task input' do
    wrapper = Rails.root.join('scripts/run_ruby_ui_comparison.fish').read

    expect(wrapper).to include('string split', 'mise exec -- bundle exec ruby scripts/compare_ruby_ui.rb $arguments')
    expect(wrapper).not_to include('eval', '--from-task-environment')
    expect(Rails.root.join('scripts/run_ruby_ui_comparison.fish')).to be_executable
  end

  it 'does not define a whole-library or secondary RubyUI comparison task' do
    expect(root_taskfile.dig('tasks', 'ruby-ui:compare:contract')).to be_nil
    expect(root_taskfile.dig('tasks', 'ruby-ui:compare').to_json).not_to include('ALL', 'component:all')
  end

  it 'defines a restore rehearsal with explicit evidence inputs and no raw restore command' do
    task = root_taskfile.dig('tasks', 'hosted-restore:rehearse') || {}
    required = task.dig('requires', 'vars') || []

    expect(required).to include(
      'DATABASE_BACKUP_ID', 'ATTACHMENT_BACKUP_ID', 'RESTORE_TARGET_ID', 'APP_IMAGE', 'TESTER',
      'HOUSEHOLD_A_ID', 'HOUSEHOLD_B_ID', 'WORM_REFERENCE', 'WORM_HEADS_JSON', 'EVIDENCE_ROOT', 'EVIDENCE_OUTPUT'
    )
    expect(task.fetch('env')).not_to have_key('RUNTIME_APP_IMAGE')
    expect(task.fetch('cmds')).to eq(['mise exec -- ruby scripts/hosted_restore_rehearsal.rb'])
    expect(task.to_json).not_to include('pg_restore', 'aws s3', 'kubectl exec')
  end

  it 'passes the exact production image reference into the production build environment' do
    expect(prod_taskfile.dig('tasks', 'build', 'env', 'APP_IMAGE_REF'))
      .to eq('{{ .APP_IMAGE_REF | default "med-tracker:local-production-build" }}')
    expect(prod_taskfile.dig('tasks', 'rebuild', 'env', 'APP_IMAGE_REF'))
      .to eq('{{ .APP_IMAGE_REF | default "med-tracker:local-production-build" }}')
    expect(internal_taskfile.dig('tasks', 'build', 'env', 'APP_IMAGE_REF'))
      .to eq('{{ .APP_IMAGE_REF | default "med-tracker:local-production-build" }}')
    expect(prod_taskfile.dig('tasks', 'build', 'cmds', 0, 'vars', 'APP_IMAGE_REF'))
      .to eq('{{ .APP_IMAGE_REF | default "med-tracker:local-production-build" }}')
  end

  it 'defines a final production-image observability characterization task' do
    task = prod_taskfile.dig('tasks', 'observability-characterization')

    expect(task.dig('vars', 'IMAGE_REF')).to eq(observability_image_reference)
    expect(task.fetch('cmds')).to include(*observability_characterization_commands)
    expect(observability_characterization_script).to include(*observability_characterization_contract)
  end

  it 'defines final production-image smokes for Disk and S3 storage' do
    disk_task = prod_taskfile.dig('tasks', 'storage-disk-smoke')
    s3_task = prod_taskfile.dig('tasks', 'storage-s3-smoke')

    expect(disk_task.fetch('cmds')).to include(*storage_smoke_commands(:disk))
    expect(s3_task.fetch('cmds')).to include(*storage_smoke_commands(:s3))
    expect(storage_smoke_script).to include(
      'ACTIVE_STORAGE_SERVICE=persistent',
      'ACTIVE_STORAGE_SERVICE=s3',
      'bin/rails db:prepare',
      'ACTIVE_STORAGE_S3_ENDPOINT=http://rustfs:9000',
      'service.upload(key, StringIO.new(payload), checksum: checksum)',
      'raise "download mismatch" unless service.download(key) == payload',
      'raise "delete failed" if service.exist?(key)',
      'S3 smoke unexpectedly mounted /app/storage'
    )
  end

  it 'exposes every bounded storage migration operator command' do
    actions = %w[
      start resume reconcile cutover-eligibility cutover rollback finalize retirement-eligibility
    ]

    actions.each do |action|
      task = prod_taskfile.dig('tasks', "storage-migration-#{action}")
      expect(task.dig('cmds', 0, 'vars', 'COMMAND')).to include(
        'DATABASE_ROLE=med_tracker_owner',
        "STORAGE_MIGRATION_ACTION=#{action.tr('-', '_')}",
        'rails storage:migration'
      )
    end
  end

  it 'exposes the bounded NHS dm+d legacy archive conversion command' do
    command = prod_taskfile.dig('tasks', 'storage-convert-nhs-dmd-archives', 'cmds', 0, 'vars', 'COMMAND')

    expect(command).to include(
      'DATABASE_ROLE=med_tracker_owner',
      'NHS_DMD_ARCHIVE_SERVICE="{{ .SERVICE }}"',
      'NHS_DMD_ARCHIVE_APPLY="{{ .APPLY }}"',
      'rails storage:convert_nhs_dmd_archives'
    )
  end

  def dev_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/dev.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def test_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/test.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def prod_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/prod.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def internal_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/internal.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def root_taskfile
    YAML.safe_load(Rails.root.join('Taskfile.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def api_client_workflow
    YAML.safe_load(api_client_workflow_source, aliases: true)
  end

  def api_client_workflow_source
    Rails.root.join('.github/workflows/api-clients.yml').read
  end

  def job_step_names(job)
    job.fetch('steps').filter_map { |step| step['name'] }
  end

  def gemfile
    Rails.root.join('Gemfile').read
  end

  def portless_script
    Rails.root.join('scripts/portless_oidc.fish').read
  end

  def swift_generator_script
    Rails.root.join('client-tools/openapi-generator/generate-swift.fish').read
  end

  def kotlin_generator_script
    Rails.root.join('client-tools/openapi-generator/generate-kotlin.fish').read
  end

  def api_client_generation_task_commands
    {
      'api-clients:generate' => root_taskfile.dig('tasks', 'api-clients:generate', 'cmds'),
      'api-clients:verify-generated' => root_taskfile.dig('tasks', 'api-clients:verify-generated', 'cmds'),
      'api-clients:kotlin' => root_taskfile.dig('tasks', 'api-clients:kotlin', 'cmds'),
      'api-clients:swift' => root_taskfile.dig('tasks', 'api-clients:swift', 'cmds'),
      'api-clients:verify' => root_taskfile.dig('tasks', 'api-clients:verify', 'cmds').map do |command|
        command.is_a?(Hash) ? command.fetch('task') : command
      end
    }
  end

  def api_client_test_task_commands
    {
      'api-clients:kotlin:test' => task_commands('api-clients:kotlin:test'),
      'api-clients:swift:test' => task_commands('api-clients:swift:test')
    }
  end

  def task_commands(name)
    root_taskfile.dig('tasks', name, 'cmds').map do |command|
      command.is_a?(Hash) ? command.fetch('task') : command
    end
  end

  def generator_image
    'openapitools/openapi-generator-cli:v7.20.0@sha256:' \
      'fa4add01856e44becf70674164df354d61bd37ba0f444d27be949801e013921b'
  end

  def dashboard_profile_script
    Rails.root.join('scripts/profile_dashboard_request.rb').read
  end

  def compose_lock_script
    Rails.root.join('scripts/with_compose_lock.rb').read
  end

  def test_preflight_script
    Rails.root.join('scripts/test_preflight.fish').read
  end

  def observability_characterization_script
    Rails.root.join('scripts/production_observability_characterization.fish').read
  end

  def storage_smoke_script
    Rails.root.join('scripts/production_storage_smoke.fish').read
  end

  def observability_image_reference
    '{{ .APP_IMAGE_REF | default "med-tracker:observability-characterization" }}'
  end

  def observability_characterization_commands
    [
      { 'task' => 'build', 'vars' => { 'APP_IMAGE_REF' => '{{ .IMAGE_REF }}' } },
      "./scripts/production_observability_characterization.fish '{{ .IMAGE_REF }}'"
    ]
  end

  def observability_characterization_contract
    [
      'OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-receiver:4318',
      'docker logs $OBSERVABILITY_CHARACTERIZATION_APP',
      'docker logs $OBSERVABILITY_CHARACTERIZATION_WORKER',
      'docker logs $OBSERVABILITY_CHARACTERIZATION_RECEIVER',
      *canary_observability_characterization_contract,
      *observability_characterization_validation_contract
    ]
  end

  def observability_characterization_validation_contract
    [
      'Application container has a repository bind mount',
      'docker exec $OBSERVABILITY_CHARACTERIZATION_RECEIVER chmod -R a+rX /var/cache/nginx/client_temp',
      'scripts/verify_otlp_trace_resources.rb',
      'Canonical request count or deployment identity is invalid',
      'Routine health-check output was not suppressed',
      'Thruster request output was not disabled',
      'Production Puma output is not producer-scoped',
      'Canonical job count or deployment identity is invalid',
      'Canonical records contain nested JSON messages'
    ]
  end

  def canary_observability_characterization_contract
    [
      'bin/rails observability:canary',
      'OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-receiver:4318/canary',
      '$2 == "/canary/v1/traces"',
      'OTLP_REQUIRED_SPAN_NAME=observability.canary',
      'Canary command did not export an enqueue-side observability trace',
      'ObservabilityCanaryJob',
      'medtracker.canary.kind',
      'medtracker.workflow.id',
      'medtracker.attempt.id',
      'Canary worker did not emit both correlated observability records'
    ]
  end

  def storage_smoke_commands(mode)
    image = "{{ .APP_IMAGE_REF | default \"med-tracker:storage-#{mode}-smoke\" }}"
    [
      { 'task' => 'build', 'vars' => { 'APP_IMAGE_REF' => '{{ .IMAGE_REF }}' } },
      "./scripts/production_storage_smoke.fish '{{ .IMAGE_REF }}' #{mode}"
    ].tap do
      expect(prod_taskfile.dig('tasks', "storage-#{mode}-smoke", 'vars', 'IMAGE_REF')).to eq(image)
    end
  end
end
