# frozen_string_literal: true

require 'json'
require 'open3'
require 'rails_helper'
require 'tempfile'
require 'tmpdir'

module NonBrowserCiShards
end

RSpec.describe NonBrowserCiShards do
  let(:workflow) { Rails.root.join('.github/workflows/ci.yml').read }
  let(:simplecov_config) { Rails.root.join('.simplecov').read }
  let(:collator) { Rails.root.join('scripts/ci/collate_simplecov.rb').read }

  it 'splits the non-browser lane into two measured file shards' do
    expect(
      [
        workflow.include?("\n        shard: [1, 2]"),
        workflow.include?('tmp/non-browser-examples.json'),
        workflow.include?('scripts/ci/non_browser_shards.mjs'),
        workflow.include?('bundle exec rspec --tag ~browser'),
        workflow.include?('COVERAGE=false bundle exec rspec --tag ~browser --dry-run'),
        Rails.root.join('scripts/ci/non_browser_timings.json').exist?
      ]
    ).to all(be(true))
  end

  it 'assigns complete spec files once with measured duration balancing' do
    payload = {
      'examples' => [
        { 'id' => './spec/a_spec.rb:1', 'file_path' => './spec/a_spec.rb' },
        { 'id' => './spec/a_spec.rb:2', 'file_path' => './spec/a_spec.rb' },
        { 'id' => './spec/b_spec.rb:1', 'file_path' => './spec/b_spec.rb' },
        { 'id' => './spec/c_spec.rb:1', 'file_path' => './spec/c_spec.rb' }
      ]
    }
    timings = { './spec/a_spec.rb' => 8.0, './spec/b_spec.rb' => 7.0, './spec/c_spec.rb' => 6.0 }
    first_output, first_error, first_status = run_shard(payload, 1, timings)
    second_output, second_error, second_status = run_shard(payload, 2, timings)

    expect([first_status, second_status]).to all(be_success)
    expect(first_output.lines).to contain_exactly("./spec/a_spec.rb\n")
    expect(second_output.lines).to contain_exactly("./spec/b_spec.rb\n", "./spec/c_spec.rb\n")
    expect([first_error, second_error]).to all(include('duration'))
  end

  it 'rejects a non-browser file without measured timing' do
    output, error, status = run_shard(
      { 'examples' => [{ 'id' => './spec/a_spec.rb:1', 'file_path' => './spec/a_spec.rb' }] },
      1,
      {}
    )

    expect(output).to be_empty
    expect(error).to include('Missing measured timing')
    expect(status).not_to be_success
  end

  it 'rejects an invalid shard index' do
    output, error, status = run_shard(
      { 'examples' => [{ 'id' => './spec/a_spec.rb:1', 'file_path' => './spec/a_spec.rb' }] },
      0,
      './spec/a_spec.rb' => 1
    )

    expect(output).to be_empty
    expect(error).to include('Shard must be within the requested range')
    expect(status).not_to be_success
  end

  it 'propagates a failing shard command' do
    stdout, = Open3.capture3('bash', '-euo', 'pipefail', '-c', 'false; echo unreachable')

    expect(stdout).to be_empty
  end

  it 'keeps shard coverage raw and collates exactly both artifacts' do
    expect(coverage_contract_values).to all(be(true))
  end

  it 'rejects missing, malformed, and duplicate SimpleCov inputs' do
    results = invalid_collator_results

    expect(results.transform_values(&:first)).to match(
      missing: include('Exactly two SimpleCov result files are required'),
      empty: include('SimpleCov result file is empty'),
      malformed: include('SimpleCov result file is malformed'),
      unexpected: include('Unexpected SimpleCov command name'),
      duplicate: include('distinct command names')
    )
    expect(results.values.map(&:last)).to all(satisfy { |status| !status.success? })
  end

  it 'requires coverage collation to be enabled explicitly' do
    output, error, status = run_collator_with_environment({ 'COVERAGE' => nil }, '{}', '{}')

    expect(output).to be_empty
    expect(error).to include('COVERAGE=true')
    expect(status).not_to be_success
  end

  it 'collates complementary shard resultsets through SimpleCov' do
    first = generate_resultset('rspec-non-system-shard-1', true)
    second = generate_resultset('rspec-non-system-shard-2', false)
    output, error, status = run_merger_with_paths(first, second)

    expect(status).to be_success
    expect(output).to include('rspec-non-system-shard-1, rspec-non-system-shard-2')
    expect(error).to be_empty
  end

  it 'fails collated results below the line, branch, and API gates' do
    first = generate_resultset('rspec-non-system-shard-1', true)
    second = generate_resultset('rspec-non-system-shard-2', false)
    output, error, status = run_collator_with_paths(first, second)

    expect(output).to be_empty
    expect(error).to include('Line coverage', 'Branch coverage', 'API branch coverage')
    expect(status).not_to be_success
  end

  it 'keeps synthetic collation outside the Rails coverage directory' do
    before = coverage_snapshot
    first = generate_resultset('rspec-non-system-shard-1', true)
    second = generate_resultset('rspec-non-system-shard-2', false)

    run_collator_with_paths(first, second)

    expect(coverage_snapshot).to eq(before)
  end

  def run_shard(payload, shard, timings)
    Tempfile.create(['non-browser-examples', '.json']) do |input|
      Tempfile.create(['non-browser-timings', '.json']) do |timing_file|
        input.write(JSON.generate(payload))
        input.flush
        timing_file.write(JSON.generate(timings))
        timing_file.flush
        Open3.capture3(
          'node', Rails.root.join('scripts/ci/non_browser_shards.mjs').to_s,
          '--input', input.path, '--timings', timing_file.path, '--shards', '2', '--shard', shard.to_s
        )
      end
    end
  end

  def run_collator(*contents)
    run_collator_with_environment({ 'COVERAGE' => 'true' }, *contents)
  end

  def run_collator_with_environment(environment, *contents)
    return collator_command(environment) if contents.empty?

    Tempfile.create(['simplecov-1', '.json']) do |first|
      Tempfile.create(['simplecov-2', '.json']) do |second|
        write_result_files([first, second], contents)
        collator_command(environment, first.path, second.path)
      end
    end
  end

  def invalid_collator_results
    {
      missing: run_collator,
      empty: run_collator('{}', '{"rspec-non-system-shard-2":{"coverage":{}}}'),
      malformed: run_collator('{not json', '{"rspec-non-system-shard-2":{}}'),
      unexpected: run_collator(
        '{"rspec-non-system":{"coverage":{}}}',
        '{"rspec-non-system-shard-2":{"coverage":{}}}'
      ),
      duplicate: run_collator(
        '{"rspec-non-system-shard-1":{"coverage":{}}}',
        '{"rspec-non-system-shard-1":{"coverage":{}}}'
      )
    }.transform_values { |result| [result[1], result[2]] }
  end

  def coverage_contract_values
    coverage_workflow_values + coverage_collator_values + [
      simplecov_config.include?("next if ENV['SIMPLECOV_SHARD'] == 'true'")
    ]
  end

  def coverage_workflow_values
    [
      workflow.include?('SIMPLECOV_COMMAND_NAME: rspec-non-system-shard-${{ matrix.shard }}'),
      workflow.include?('SIMPLECOV_SHARD: true'),
      workflow.include?('path: coverage/.resultset.json'),
      workflow.include?('include-hidden-files: true'),
      workflow.include?('if-no-files-found: error'),
      workflow.include?('simplecov-non-system-1'),
      workflow.include?('simplecov-non-system-2'),
      workflow.include?('scripts/ci/collate_simplecov.rb')
    ]
  end

  def coverage_collator_values
    [
      collator.include?('SimpleCov.collate'),
      collator.include?("ENV['COVERAGE'] == 'true'"),
      collator.include?("ENV.delete('SIMPLECOV_SHARD')")
    ]
  end

  def collator_command(environment, *paths)
    Dir.mktmpdir('simplecov-collate') do |coverage_dir|
      Open3.capture3(
        environment.merge(
          'SIMPLECOV_COVERAGE_DIR' => coverage_dir,
          'SIMPLECOV_SHARD' => nil
        ),
        'bundle', 'exec', 'ruby', Rails.root.join('scripts/ci/collate_simplecov.rb').to_s,
        *paths, chdir: Rails.root.to_s
      )
    end
  end

  def write_result_files(files, contents)
    files.zip(contents).each do |file, content|
      file.write(content)
      file.flush
    end
  end

  def run_merger_with_paths(*paths)
    Open3.capture3(
      'bundle', 'exec', 'ruby', '-e',
      "require 'simplecov'; puts SimpleCov::ResultMerger.merge_results(*ARGV).command_name",
      *paths, chdir: Rails.root.to_s
    )
  end

  def run_collator_with_paths(*paths)
    collator_command({ 'COVERAGE' => 'true' }, *paths)
  end

  def generate_resultset(command_name, value)
    directory = Dir.mktmpdir('simplecov-shard')
    source = Tempfile.new(['simplecov-source', '.rb'])
    source.write(resultset_source(value))
    source.close
    script = resultset_script(directory, source.path, command_name)
    _output, error, status = Open3.capture3(
      resultset_environment, 'bundle', 'exec', 'ruby', '-e', script, chdir: Rails.root.to_s
    )
    raise error unless status.success?

    File.join(directory, '.resultset.json')
  end

  def resultset_environment
    { 'COVERAGE' => 'true', 'SIMPLECOV_SHARD' => 'true' }
  end

  def resultset_source(value)
    "value = #{value}\nif value\n  puts :covered\nelse\n  puts :uncovered\nend\n"
  end

  def resultset_script(directory, source_path, command_name)
    <<~RUBY
      require 'simplecov'
      SimpleCov.start do
        coverage_dir #{directory.inspect}
        command_name #{command_name.inspect}
        enable_coverage :branch
      end
      load #{source_path.inspect}
    RUBY
  end

  def coverage_snapshot
    coverage_root = Rails.root.join('coverage')
    return {} unless coverage_root.directory?

    coverage_root.glob('**/*', File::FNM_DOTMATCH).each_with_object({}) do |path, snapshot|
      next if path.basename.to_s.in?(%w[. ..])

      snapshot[path.to_s] = [path.mtime, path.file? ? path.size : nil]
    end
  end
end
