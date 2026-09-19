require 'spec_helper'
require 'open3'
require 'json'
require 'rubocop'

RSpec.describe RuboCop::CLI do
  let(:root) { File.expand_path('../../..', __dir__) }

  def inspect_source(cop, source)
    Open3.capture3(
      'bundle', 'exec', 'rubocop', '--config', File.join(root, '.rubocop.yml'),
      '--only', cop, '--format', 'json', '--stdin', 'app/services/lint_example.rb',
      stdin_data: source, chdir: root
    )
  end

  it 'rejects an unnecessary intermediate collection' do
    output, error, status = inspect_source('Performance/MapCompact', "values.map(&:name).compact\n")

    expect(status.exitstatus).to eq(1), error
    expect(JSON.parse(output).fetch('files').flat_map { |file| file.fetch('offenses') })
      .to include(include('cop_name' => 'Performance/MapCompact'))
  end

  it 'keeps convention offences blocking under preview' do
    output, error, status = inspect_source('Layout/SpaceAroundOperators', "value=1\n")

    expect(status.exitstatus).to eq(1), error
    expect(JSON.parse(output).fetch('summary').fetch('offense_count')).to be_positive
  end

  it 'keeps application entry points in the inspection scope' do
    output, error, status = Open3.capture3(
      'bundle', 'exec', 'rubocop', '--config', File.join(root, '.rubocop.yml'),
      '--list-target-files', chdir: root
    )

    expect(status).to be_success, error
    expect(output.lines.map { |path| File.expand_path(path.strip, root) }).to include(
      File.join(root, 'bin/audit-exporter'), File.join(root, 'bin/jobs')
    )
  end
end
