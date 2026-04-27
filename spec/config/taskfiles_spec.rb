# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Taskfiles' do
  let(:dev_taskfile) do
    YAML.safe_load(Rails.root.join('Taskfiles/dev.yml').read, aliases: true, permitted_classes: [Symbol])
  end
  let(:test_taskfile) do
    YAML.safe_load(Rails.root.join('Taskfiles/test.yml').read, aliases: true, permitted_classes: [Symbol])
  end
  let(:portless_script) { Rails.root.join('scripts/portless_oidc.fish').read }

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
    expect(portless_script).to include('curl --silent --show-error --head --fail --max-time 5 $base_url')
    expect(portless_script).to include('Portless did not respond at $base_url.')
    expect(portless_script).to include('portless proxy start --port 443 --https --force')
  end

  it 'uses test-scoped OIDC variables for the Portless test task' do
    expect(portless_script).to include('set -lx TEST_APP_URL $base_url')
    expect(portless_script).to include('set -lx TEST_OIDC_CLIENT_ID $OIDC_CLIENT_ID')
    expect(portless_script).to include('set -lx TEST_OIDC_REDIRECT_URI $callback_url')
  end
end
