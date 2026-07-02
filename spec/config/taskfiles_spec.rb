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

    expect(compose_project).to include('git rev-parse --path-format=absolute --git-common-dir')
    expect(compose_project).to include('git hash-object --stdin')
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
end
