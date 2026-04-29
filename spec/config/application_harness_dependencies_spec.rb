# frozen_string_literal: true

require 'rails_helper'
require 'json'

module ApplicationHarnessDependencies
end

RSpec.describe ApplicationHarnessDependencies do
  let(:gemfile) { Rails.root.join('Gemfile').read }
  let(:dockerfile) { Rails.root.join('Dockerfile').read }
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

  it 'does not install Lighthouse with the regular Node dependency set' do
    expect(package_json.fetch('devDependencies')).not_to include('lighthouse')
  end

  it 'installs target-specific bundle groups in Docker' do
    expect(dockerfile).to include('BUNDLE_WITHOUT')
    expect(dockerfile).to include('BUNDLE_WITH')
  end
end
