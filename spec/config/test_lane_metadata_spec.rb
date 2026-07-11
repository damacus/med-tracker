# frozen_string_literal: true

require 'rails_helper'

module TestLaneMetadata
  PLAYWRIGHT_API_PATTERN = /driven_by\(:playwright\)|with_playwright_page|execute_script|evaluate_script|resize_to/
end

RSpec.describe TestLaneMetadata do
  let(:rails_helper) { Rails.root.join('spec/rails_helper.rb').read }
  let(:capybara_support) { Rails.root.join('spec/support/capybara.rb').read }
  let(:ci_workflow) { Rails.root.join('.github/workflows/ci.yml').read }

  it 'does not infer browser requirements from spec directories or types' do
    expect(rails_helper).not_to include('file_path: %r{/spec/(system|features|views)/}')
    expect(rails_helper).not_to include('define_derived_metadata(type: :system)')
    expect(rails_helper).not_to include('define_derived_metadata(type: :feature)')
  end

  it 'maps explicit JavaScript metadata into the browser lane' do
    expect(rails_helper).to include('config.define_derived_metadata(js: true)')
    expect(rails_helper).to include('metadata[:browser] = true')
  end

  it 'selects the Capybara driver from explicit browser metadata' do
    expect(capybara_support).to include('example.metadata[:browser] ? :playwright : :rack_test')
  end

  it 'runs tagged browser examples from the complete spec tree in CI' do
    expect(ci_workflow).to include("find spec -name '*_spec.rb'")
    expect(ci_workflow).to include('bundle exec rspec --tag browser')
    expect(ci_workflow).to include('bundle exec rspec --format RSpec::Github::Formatter --tag ~browser')
  end

  it 'requires explicit metadata in specs that use Playwright-only APIs' do
    offenders = Rails.root.glob('spec/{features,system,views}/**/*_spec.rb').reject do |path|
      contents = path.read
      next true unless contents.match?(described_class::PLAYWRIGHT_API_PATTERN)

      contents.match?(/:browser|:js|browser: true|js: true/)
    end

    expect(offenders).to be_empty
  end

  it 'does not allow arbitrary sleeps in browser-capable specs' do
    offenders = Rails.root.glob('spec/{features,system,views}/**/*_spec.rb').select do |path|
      path.readlines.reject { |line| line.lstrip.start_with?('#') }.any? do |line|
        line.match?(/\bsleep(?:\s|\()/)
      end
    end

    expect(offenders).to be_empty
  end
end
