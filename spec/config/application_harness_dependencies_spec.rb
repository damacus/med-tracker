# frozen_string_literal: true

require 'rails_helper'
require 'json'

module ApplicationHarnessDependencies
end

RSpec.describe ApplicationHarnessDependencies do
  let(:package_json) { JSON.parse(Rails.root.join('package.json').read) }

  it 'keeps runtime-only gems out of development and test bundles' do
    expect(gemfile.index("gem 'ruby_ui'")).to be < gemfile.index('group :development')
    expect(gemfile).to include('group :tools do')
    expect(gemfile).to include('group :production do')
  end

  it 'uses explicit OpenTelemetry instrumentation gems instead of the all bundle' do
    expect(gemfile).not_to include("gem 'opentelemetry-instrumentation-all'")
    expect(gemfile).to include("gem 'opentelemetry-instrumentation-rails'")
    expect(gemfile).to include("gem 'opentelemetry-instrumentation-rack'")
    expect(gemfile).to include("gem 'opentelemetry-instrumentation-pg'")
    expect(gemfile).to include("gem 'opentelemetry-instrumentation-net_http'")
  end

  it 'removes unused local test tooling from the bundle' do
    expect(gemfile).not_to include("gem 'parallel_tests'")
    expect(gemfile).not_to include("gem 'rails-controller-testing'")
  end

  it 'keeps code quality tooling available in local bundles' do
    expect(gemfile).to include("gem 'simplecov', require: false")
    expect(gemfile).to include("gem 'rubycritic', require: false")
    expect(taskfile).to include('rubycritic:')
  end

  it 'starts SimpleCov before Rails loads when coverage is enabled' do
    expect(spec_helper.index("require 'simplecov'")).to be < spec_helper.index("require 'webmock/rspec'")
    expect(spec_helper.index('SimpleCov.start')).to be < spec_helper.index("require 'webmock/rspec'")
    expect(simplecov_config).to include('SimpleCov.configure')
    expect(simplecov_config).not_to include('SimpleCov.start')
    expect(Rails.root.join('.simplecov')).to exist
  end

  it 'enables SimpleCov during CI test runs' do
    expect(ci_workflow).to include('COVERAGE: true')
    expect(compose_yaml).to include('COVERAGE: ${COVERAGE:-false}')
  end

  it 'does not install Lighthouse with the regular Node dependency set' do
    expect(package_json.fetch('devDependencies')).not_to include('lighthouse')
  end

  it 'installs target-specific bundle groups in Docker' do
    expect(dockerfile).to include('BUNDLE_WITHOUT')
    expect(dockerfile).to include('BUNDLE_WITH')
  end

  it 'keeps development database headers out of the production image stage' do
    app_stage = dockerfile.split('FROM ruby:4.0.4-slim-trixie AS app').fetch(1)

    expect(app_stage).to include('libpq5')
    expect(app_stage).not_to include('libpq-dev')
    expect(app_stage).not_to match(/apt-get install[^\n]+curl/)
  end

  def gemfile
    Rails.root.join('Gemfile').read
  end

  def dockerfile
    Rails.root.join('Dockerfile').read
  end

  def compose_yaml
    Rails.root.join('compose.yaml').read
  end

  def taskfile
    Rails.root.join('Taskfile.yml').read
  end

  def spec_helper
    Rails.root.join('spec/spec_helper.rb').read
  end

  def simplecov_config
    Rails.root.join('.simplecov').read
  end

  def ci_workflow
    Rails.root.join('.github/workflows/ci.yml').read
  end
end
