# frozen_string_literal: true

require 'rails_helper'
require 'simplecov'
require 'open3'
require 'tmpdir'

RSpec.describe SimpleCov do
  let(:simplecov_config) { Rails.root.join('.simplecov').read }
  let(:taskfile) { Rails.root.join('Taskfile.yml').read }
  let(:lighthouse_tasks) { Rails.root.join('Taskfiles/lighthouse.yml').read }
  let(:ci_workflow) { Rails.root.join('.github/workflows/ci.yml').read }

  it 'skips formatting when another process has already consumed the SimpleCov result' do
    result_index = simplecov_config.index('result = SimpleCov.result')
    guard_index = simplecov_config.index('next unless result')
    format_index = simplecov_config.index('result.format!')

    expect(result_index).to be < guard_index
    expect(guard_index).to be < format_index
  end

  it 'keeps the line, branch, and API branch coverage gates' do
    expect(simplecov_config).to include('minimum_coverage line: 90, branch: 75')
    expect(simplecov_config).to include("next unless ENV['COVERAGE'] == 'true'")
    expect(simplecov_config).to include("result.groups.fetch('API')")
    expect(simplecov_config).to include('api_percent >= 90.0')
    expect(simplecov_config).to include('Kernel.exit SimpleCov::ExitCodes::MINIMUM_COVERAGE')
  end

  it 'enforces coverage without a shard bypass' do
    expect(simplecov_config).not_to include('SIMPLECOV_SHARD')
  end

  it 'passes a fully covered run with coverage enabled' do
    _output, error, status = run_coverage('choose(true); choose(false)')

    expect(status).to be_success
    expect(error).to include('100.00%')
    expect(error).not_to include('below')
  end

  it 'fails an incomplete run against the line, branch, and API gates' do
    _output, error, status = run_coverage('choose(true)')

    expect(status).not_to be_success
    expect(error).to include('Line coverage', 'Branch coverage', 'API branch coverage')
  end

  it 'targets application code by default in RubyCritic' do
    rubycritic_task = taskfile.split("\n  rubycritic:\n", 2).last.split("\n  playwright:\n", 2).first

    expect(rubycritic_task).to include("target: '{{ .TARGET | default \"app\" }}'")
    expect(rubycritic_task).not_to include('default "app spec"')
  end

  it 'enables Lighthouse on the current runtime stack' do
    job = lighthouse_job

    expect(ci_workflow).to include("\n  lighthouse:\n")
    expect(job).to include('image: postgres:18-alpine')
    expect(job).to include('ruby-version: "4.0.6"')
    expect(job).to include('uses: actions/setup-node@v7')
    expect(job).to include('node-version: "24"')
  end

  it 'pins the Lighthouse setup actions' do
    job = lighthouse_job

    expect(job).to include('uses: browser-actions/setup-chrome@2e1d749697dd1612b833dba4a722266286fbefcd')
    expect(job).to include('uses: arduino/setup-task@c0bc642852239c2689f73f4ea6459c29405f3c52')
  end

  it 'audits public and authenticated mobile workflows and always retains reports' do
    job = lighthouse_job

    expect(job).to include('URL=http://localhost:3000/login')
    expect(job).to include('URL=http://localhost:3000/households/fixture-household/dashboard')
    expect(job).to include('EXTRA_HEADERS=')
  end

  it 'always retains Lighthouse reports' do
    job = lighthouse_job

    expect(job).to include('if: always()')
    expect(job).to include('uses: actions/upload-artifact@v7')
    expect(job).to include('lighthouse-*.report.json')
    expect(job).to include('lighthouse-*.report.html')
  end

  it 'enforces explicit mobile score thresholds and authenticated headers' do
    expect(lighthouse_tasks).to include("PERF_THRESHOLD: '{{ .PERF_THRESHOLD | default \"60\" }}'")
    expect(lighthouse_tasks).to include("A11Y_THRESHOLD: '{{ .A11Y_THRESHOLD | default \"90\" }}'")
    expect(lighthouse_tasks).to include("BP_THRESHOLD: '{{ .BP_THRESHOLD | default \"90\" }}'")
    expect(lighthouse_tasks).to include("EXTRA_HEADERS: '{{ .EXTRA_HEADERS | default \"\" }}'")
    expect(lighthouse_tasks).to include("--extra-headers='{{ .EXTRA_HEADERS }}'")
  end

  def lighthouse_job
    ci_workflow.split("\n  lighthouse:\n", 2).last
  end

  def run_coverage(invocation)
    Dir.mktmpdir('ci-coverage') do |directory|
      source = File.join(directory, 'app/controllers/api/example.rb')
      FileUtils.mkdir_p(File.dirname(source))
      File.write(source, "def choose(value)\n  if value\n    :yes\n  else\n    :no\n  end\nend\n#{invocation}\n")
      script = <<~RUBY
        require 'simplecov'
        SimpleCov.root #{directory.inspect}
        load #{Rails.root.join('.simplecov').to_s.inspect}
        SimpleCov.start
        load #{source.inspect}
      RUBY
      Open3.capture3({ 'COVERAGE' => 'true' }, 'bundle', 'exec', 'ruby', '-e', script)
    end
  end
end
