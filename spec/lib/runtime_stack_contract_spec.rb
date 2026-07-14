# frozen_string_literal: true

require 'rails_helper'

module RuntimeStackContract
  module_function

  def expected_versions
    {
      ruby: '4.0.6',
      node: '24',
      rails: '8.1.3',
      postgres: '18',
      bundler: '4.0.3',
      playwright: '1.61'
    }
  end
end

RSpec.describe RuntimeStackContract do
  let(:expected_versions) { described_class.expected_versions }

  it 'keeps Ruby runtime metadata aligned across local, Docker, CI, and agent docs' do
    expected_ruby_version = expected_versions.fetch(:ruby)

    expect(read_file('.ruby-version').strip).to eq("ruby-#{expected_ruby_version}")
    expect(mise_version('ruby')).to eq(expected_ruby_version)
    expect(read_file('Dockerfile')).to start_with("FROM rubylang/ruby:#{expected_ruby_version}-resolute AS base")
    expect(ci_values('ruby-version')).to contain_exactly(expected_ruby_version)
    agent_guide_paths.each do |path|
      expect(read_file(path)).to include("- Ruby #{expected_ruby_version}")
    end
  end

  it 'keeps Node runtime metadata aligned across Docker and CI' do
    expected_node_version = expected_versions.fetch(:node)

    expect(mise_version('node')).to eq(expected_node_version)
    expect(read_file('Dockerfile')).to include("https://deb.nodesource.com/node_#{expected_node_version}.x")
    expect(ci_values('node-version')).to contain_exactly(expected_node_version)
  end

  it 'keeps locked dependency metadata aligned with the target stack' do
    expect(locked_gem_version('rails')).to eq(expected_versions.fetch(:rails))
    expect(bundler_version).to eq(expected_versions.fetch(:bundler))
    expect(read_file('compose/base.yml')).to include("postgres:#{expected_versions.fetch(:postgres)}-alpine")
    expect(playwright_version).to start_with("#{expected_versions.fetch(:playwright)}.")
  end

  it 'loads the MCP Ruby SDK server primitives used by the hosted MCP endpoint' do
    require 'mcp'

    expect(MCP::Server).to be_a(Class)
    expect(MCP::Tool).to be_a(Class)
    expect(MCP::Server::Transports::StreamableHTTPTransport).to be_a(Class)
  end

  it 'documents locked dependency versions in both agent guides' do
    agent_guide_paths.each do |path|
      document = read_file(path)

      expect(document).to include("- Rails #{expected_versions.fetch(:rails)}")
      expect(document).to include("- PostgreSQL #{expected_versions.fetch(:postgres)}")
      expect(document).to include("- Bundler #{expected_versions.fetch(:bundler)}")
      expect(document).to include("- Playwright #{expected_versions.fetch(:playwright)}")
    end
  end

  def read_file(path)
    Rails.root.join(path).read
  end

  def agent_guide_paths
    paths = ['agents.md']
    uppercase_path = Rails.root.join('AGENTS.md')

    paths << 'AGENTS.md' if uppercase_path.exist? && !File.identical?(uppercase_path, Rails.root.join('agents.md'))
    paths
  end

  def uncommented_file(path)
    read_file(path).lines.grep_v(/\A\s*#/).join
  end

  def ci_values(key)
    uncommented_file('.github/workflows/ci.yml')
      .scan(/#{Regexp.escape(key)}:\s*["']?([^"'\s]+)["']?/)
      .flatten
      .uniq
  end

  def mise_version(tool)
    read_file('mise.toml')[/^#{Regexp.escape(tool)} = "([^"]+)"$/, 1]
  end

  def locked_gem_version(name)
    read_file('Gemfile.lock')[/^    #{Regexp.escape(name)} \(([^)]+)\)/, 1]
  end

  def bundler_version
    read_file('Gemfile.lock').split(/^BUNDLED WITH\n/).last.strip
  end

  def playwright_version
    JSON.parse(read_file('package-lock.json')).fetch('packages').fetch('node_modules/playwright').fetch('version')
  end
end
