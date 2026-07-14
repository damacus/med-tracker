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

  it 'serializes Docker Compose runs within each worktree environment' do
    command = internal_taskfile.dig('tasks', 'run', 'cmds', 0)

    expect(command).to include('./scripts/with_compose_lock.rb "{{ .COMPOSE_PROJECT }}-{{ .ENVIRONMENT }}"')
    expect(compose_lock_script).to include(
      'lock.flock(File::LOCK_EX)',
      'Process.spawn(*command)',
      'Process.wait2(pid)'
    )
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

  it 'defines a restore rehearsal with explicit evidence inputs and no raw restore command' do
    task = root_taskfile.dig('tasks', 'hosted-restore:rehearse') || {}
    required = task.dig('requires', 'vars') || []

    expect(required).to include(
      'DATABASE_BACKUP_ID', 'ATTACHMENT_BACKUP_ID', 'RESTORE_TARGET_ID', 'APP_IMAGE', 'TESTER',
      'HOUSEHOLD_A_ID', 'HOUSEHOLD_B_ID', 'WORM_REFERENCE', 'WORM_HEADS_JSON', 'EVIDENCE_OUTPUT'
    )
    expect(task.fetch('cmds')).to eq(['mise exec -- ruby scripts/hosted_restore_rehearsal.rb'])
    expect(task.to_json).not_to include('pg_restore', 'aws s3', 'kubectl exec')
  end

  def dev_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/dev.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def test_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/test.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def internal_taskfile
    YAML.safe_load(Rails.root.join('Taskfiles/internal.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def root_taskfile
    YAML.safe_load(Rails.root.join('Taskfile.yml').read, aliases: true, permitted_classes: [Symbol])
  end

  def gemfile
    Rails.root.join('Gemfile').read
  end

  def portless_script
    Rails.root.join('scripts/portless_oidc.fish').read
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
end
