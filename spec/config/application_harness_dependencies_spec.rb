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

  it 'limits the dummy application URL to production asset compilation' do
    assets_stage = docker_stage('assets')
    app_stage = docker_stage('app')

    expect(assets_stage).to include(
      'RUN APP_URL=https://assets-build.invalid SECRET_KEY_BASE_DUMMY=1 rails assets:precompile'
    )
    expect(app_stage).not_to include('APP_URL=')
  end

  it 'provides an overridable URL to the production-style Compose environment' do
    expect(compose_yaml).to include('APP_URL: ${APP_URL:-http://localhost}')
  end

  it 'migrates a new production database without loading tenant seeds' do
    production_services = compose_yaml.split('  # === Production (profile: prod) ===', 2).last
    migrate_service = production_services.split("  migrate-prod:\n", 2).last.split("  web-prod:\n", 2).first

    expect(migrate_service).to include('command: bin/rails db:migrate')
    expect(migrate_service).not_to include('db:prepare')
    expect(compose_yaml.scan('command: bin/rails db:prepare').size).to eq(2)
  end

  it 'keeps Debian package installs out of the shared Docker base stage' do
    base_stage = docker_stage('base')

    expect(base_stage).not_to include('apt-get install')
    expect(base_stage).not_to include('apt-get upgrade')
  end

  it 'installs native gem build packages only in the Docker gem builder stage' do
    gem_builder_stage = docker_stage('gem_builder')

    expect(gem_builder_stage).to include(expected_gem_builder_package_install)
    expect(gem_builder_stage).not_to include('curl')
    expect(gem_builder_stage).not_to include('git')
    expect(gem_builder_stage).not_to include('unzip')
    expect(gem_builder_stage).not_to include('apt-get upgrade')
  end

  it 'copies only required Rails paths into the development Docker image' do
    development_stage = docker_stage('development')

    expected_development_copy_lines.each do |copy_line|
      expect(development_stage).to include(copy_line)
    end

    expect(development_stage).not_to include('COPY --chown=ruby:ruby . .')
  end

  it 'installs the health-check client in the development Docker image' do
    development_stage = docker_stage('development')

    expect(development_stage).to include('apt-get install -y --no-install-recommends curl')
  end

  it 'keeps the OpenTelemetry SDK available to development boot' do
    dependency = gemfile_dependencies.fetch('opentelemetry-sdk')

    expect(dependency.groups).to include(:default)
  end

  it 'copies only required Rails and test paths into the test Docker image' do
    test_stage = docker_stage('test')

    expected_test_copy_lines.each do |copy_line|
      expect(test_stage).to include(copy_line)
    end

    expect(test_stage).not_to include('COPY --chown=ruby:ruby . .')
  end

  it 'installs only required runtime packages in the production Docker image' do
    app_stage = docker_stage('app')

    expect(app_stage).to include('apt-get install -y --no-install-recommends ca-certificates curl libpq5 unzip')
    expect(forbidden_production_dependencies.select { |dependency| app_stage.include?(dependency) }).to be_empty
  end

  it 'copies only required runtime paths into the production Docker image' do
    app_stage = docker_stage('app')

    expected_production_copy_lines.each do |copy_line|
      expect(app_stage).to include(copy_line)
    end

    expect(app_stage).not_to include('COPY --chown=ruby:ruby . .')
    expect(app_stage).not_to include('RUN chmod 0755 bin/*')
  end

  def gemfile
    Rails.root.join('Gemfile').read
  end

  def gemfile_dependencies
    Bundler::Dsl
      .evaluate(Rails.root.join('Gemfile').to_s, Rails.root.join('Gemfile.lock').to_s, {})
      .dependencies
      .index_by(&:name)
  end

  def dockerfile
    Rails.root.join('Dockerfile').read
  end

  def docker_stage(name)
    lines = dockerfile.lines
    start_index = lines.find_index { |line| line.match?(/^FROM .+ AS #{Regexp.escape(name)}$/) }
    raise KeyError, "Docker stage not found: #{name}" unless start_index

    end_index = lines[(start_index + 1)..].find_index { |line| line.start_with?('FROM ') }
    if end_index
      lines[start_index, end_index + 1].join
    else
      lines[start_index..].join
    end
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

  def expected_development_copy_lines
    [
      'COPY --chown=ruby:ruby app/ ./app/',
      'COPY --chown=ruby:ruby bin/ ./bin/',
      'COPY --chown=ruby:ruby config/ ./config/',
      'COPY --chown=ruby:ruby db/ ./db/',
      'COPY --chown=ruby:ruby lib/ ./lib/',
      'COPY --chown=ruby:ruby public/ ./public/',
      'COPY --chown=ruby:ruby config.ru Rakefile ./'
    ]
  end

  def expected_test_copy_lines
    expected_development_copy_lines + [
      'COPY --chown=ruby:ruby spec/ ./spec/',
      'COPY --chown=ruby:ruby .rspec .simplecov ./'
    ]
  end

  def expected_gem_builder_package_install
    'apt-get install -y --no-install-recommends build-essential libpq-dev libyaml-dev'
  end

  def forbidden_production_dependencies
    %w[build-essential git libpq-dev libyaml-dev] + ['apt-get upgrade']
  end

  def expected_production_copy_lines
    [
      'COPY --chown=ruby:ruby --from=assets /usr/local/bundle /usr/local/bundle',
      'COPY --chown=ruby:ruby --from=assets /app/app /app/app',
      'COPY --chown=ruby:ruby --from=assets /app/bin /app/bin',
      'COPY --chown=ruby:ruby --from=assets /app/config /app/config',
      'COPY --chown=ruby:ruby --from=assets /app/db /app/db',
      'COPY --chown=ruby:ruby --from=assets /app/lib /app/lib',
      'COPY --chown=ruby:ruby --from=assets /app/public /app/public',
      'COPY --chown=ruby:ruby --from=assets /app/config.ru /app/Rakefile /app/',
      'COPY --chown=ruby:ruby --from=assets /app/Gemfile /app/Gemfile.lock /app/'
    ]
  end
end
