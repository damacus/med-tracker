# frozen_string_literal: true

require 'json'
require 'open3'
require 'rails_helper'
require 'tempfile'

module CiRuntimeLanes
end

RSpec.describe CiRuntimeLanes do
  let(:workflow) { Rails.root.join('.github/workflows/ci.yml').read }
  let(:dockerfile) { Rails.root.join('Dockerfile').read }
  let(:gemfile_lock) { Rails.root.join('Gemfile.lock').read }
  let(:package_lock) { JSON.parse(Rails.root.join('package-lock.json').read) }
  let(:policy) { JSON.parse(Rails.root.join('scripts/ci/policy.json').read) }

  it 'aligns Ruby and npm Playwright versions without compatibility aliases' do
    ruby_version = gemfile_lock[/^    playwright-ruby-client \(([^)]+)\)/, 1]
    npm_version = package_lock.fetch('packages').fetch('node_modules/playwright').fetch('version')

    expect(ruby_version).to eq('1.62.0')
    expect(npm_version).to eq('1.63.0')
    expect(dockerfile).not_to include('ln -s')
  end

  it 'runs Lighthouse independently for UI-relevant changes and gates its result' do
    lighthouse_job = workflow.split("\n  lighthouse:\n", 2).last.split("\n  ci_success:\n", 2).first
    ui_rule = policy.fetch('rules').find { |rule| rule.fetch('pattern').include?('app/(components') }

    expect(lighthouse_conditions(lighthouse_job, ui_rule)).to all(be(true))
  end

  it 'builds browser shards from tagged examples with measured timing data' do
    expect(
      [
        workflow.include?('bundle exec rspec --tag browser --dry-run --format json'),
        workflow.include?('scripts/ci/browser_shards.mjs'),
        workflow.include?('set -euo pipefail'),
        workflow.exclude?('< <(node scripts/ci/browser_shards.mjs'),
        workflow.exclude?('find spec -name'),
        Rails.root.join('scripts/ci/browser_timings.json').exist?
      ]
    ).to all(be(true))
  end

  it 'assigns every browser example once with deterministic duration balancing' do
    payload = browser_payload
    timings = {
      './spec/a_spec.rb' => 8.0,
      './spec/b_spec.rb' => 7.0,
      './spec/c_spec.rb' => 6.0,
      './spec/d_spec.rb' => 5.0
    }
    first_output, first_error, first_status = run_browser_shards(payload, 1, timings: timings)
    second_output, second_error, second_status = run_browser_shards(payload, 2, timings: timings)

    expect([first_status, second_status]).to all(be_success)
    expect(first_output.lines).to contain_exactly("./spec/a_spec.rb[1:1]\n", "./spec/d_spec.rb[1:1]\n")
    expect(second_output.lines).to contain_exactly("./spec/b_spec.rb[1:1]\n", "./spec/c_spec.rb[1:1]\n")
    expect([first_error, second_error]).to all(include('"duration":13'))
  end

  it 'rejects browser examples without measured timings' do
    output, error, status = run_browser_shards(browser_payload, 1, timings: { './spec/a_spec.rb' => 8 })

    expect(output).to be_empty
    expect(error).to include('Missing measured timing')
    expect(status).not_to be_success
  end

  it 'rejects browser examples with invalid measured timings' do
    output, error, status = run_browser_shards(browser_payload, 1, timings: { './spec/a_spec.rb' => 0 })

    expect(output).to be_empty
    expect(error).to include('Missing measured timing')
    expect(status).not_to be_success
  end

  it 'caps Android assembly workers without changing the existing variants' do
    taskfile = Rails.root.join('mobile/android/Taskfile.yml').read

    expect(taskfile).to include('--max-workers=1')
    gradle_properties = Rails.root.join('mobile/android/gradle.properties').read

    expect(taskfile).to include('--stacktrace')
    expect(gradle_properties).to include('org.gradle.jvmargs=-Xmx4096m')
    expect(taskfile).to include(
      ':phone:assembleDebug', ':phone:assembleStaging', ':phone:assembleRelease',
      ':wear:assembleDebug', ':wear:assembleStaging', ':wear:assembleRelease',
      'release:check', 'wear:boundary', 'api:check'
    )
  end

  it 'keeps the Lighthouse aggregate gate closed for failed, cancelled, or missing results' do
    outcomes = {
      success: gate_errors('success', selected: true),
      failure: gate_errors('failure', selected: true),
      cancelled: gate_errors('cancelled', selected: true),
      missing: gate_errors(nil, selected: true),
      unselected: gate_errors('skipped', selected: false)
    }

    expect(outcomes[:success]).to eq([])
    expect(outcomes[:failure].any? { |error| error.include?('lighthouse') }).to be(true)
    expect(outcomes[:cancelled].any? { |error| error.include?('lighthouse') }).to be(true)
    expect(outcomes[:missing].any? { |error| error.include?('lighthouse') }).to be(true)
    expect(outcomes[:unselected]).to eq([])
  end

  it 'requires collated coverage to succeed when Rails tests are selected' do
    outcomes = {
      failure: coverage_gate_errors('failure'),
      cancelled: coverage_gate_errors('cancelled'),
      missing: coverage_gate_errors(nil)
    }

    expect(outcomes.values).to all(satisfy { |errors| errors.any? { |error| error.include?('coverage') } })
  end

  it 'classifies representative UI and non-UI paths through the executable classifier' do
    ui = classify_paths('app/components/dashboard.rb').fetch('selected')
    non_ui = classify_paths('app/models/medication.rb').fetch('selected')

    expect([ui.fetch('lighthouse'), non_ui.fetch('lighthouse')]).to eq([true, false])
  end

  def run_browser_shards(payload, shard, timings: {})
    Tempfile.create(['browser-examples', '.json']) do |input|
      Tempfile.create(['browser-timings', '.json']) do |timing_file|
        input.write(JSON.generate(payload))
        input.flush
        timing_file.write(JSON.generate(timings))
        timing_file.flush
        Open3.capture3(
          'node', Rails.root.join('scripts/ci/browser_shards.mjs').to_s,
          '--input', input.path, '--timings', timing_file.path, '--shards', '2', '--shard', shard.to_s
        )
      end
    end
  end

  def browser_payload
    {
      'examples' => [
        { 'id' => './spec/a_spec.rb[1:1]', 'file_path' => './spec/a_spec.rb', 'run_time' => 8.0 },
        { 'id' => './spec/b_spec.rb[1:1]', 'file_path' => './spec/b_spec.rb', 'run_time' => 7.0 },
        { 'id' => './spec/c_spec.rb[1:1]', 'file_path' => './spec/c_spec.rb', 'run_time' => 6.0 },
        { 'id' => './spec/d_spec.rb[1:1]', 'file_path' => './spec/d_spec.rb', 'run_time' => 5.0 }
      ]
    }
  end

  def lighthouse_conditions(lighthouse_job, ui_rule)
    [
      lighthouse_job.include?('needs: changes'), lighthouse_job.exclude?('test_non_system'),
      workflow.include?('lighthouse: ${{ steps.filter.outputs.lighthouse }}'),
      workflow.include?('      - lighthouse'), policy.fetch('jobs').fetch('lighthouse') == ['lighthouse'],
      ui_rule.fetch('suites').include?('lighthouse')
    ]
  end

  def classify_paths(path)
    stdout, stderr, status = Open3.capture3(
      'node', '--input-type=module', '-e',
      "import { classify } from './scripts/ci/classify.mjs'; console.log(JSON.stringify(classify([process.argv[1]])));",
      path,
      chdir: Rails.root.to_s
    )
    raise stderr unless status.success?

    JSON.parse(stdout)
  end

  def gate_errors(result, selected:)
    evaluate_gate(gate_needs(result, selected))
  end

  def coverage_gate_errors(result)
    needs = gate_needs('success', true)
    needs['changes']['outputs']['rails'] = 'true'
    policy.fetch('jobs').fetch('rails').each { |job| needs[job] = { 'result' => 'success' } }
    result ? needs['coverage'] = { 'result' => result } : needs.delete('coverage')

    evaluate_gate(needs)
  end

  def evaluate_gate(needs)
    stdout, stderr, status = Open3.capture3(
      'node', '--input-type=module', '-e',
      gate_script,
      JSON.generate(needs),
      chdir: Rails.root.to_s
    )
    raise stderr unless status.success?

    JSON.parse(stdout)
  end

  def gate_needs(result, selected)
    selections = policy.fetch('jobs').keys.index_with { 'false' }
    selections['lighthouse'] = selected ? 'true' : 'false'
    jobs = policy.fetch('jobs').values.flatten.index_with { { 'result' => 'skipped' } }
    result ? jobs['lighthouse'] = { 'result' => result } : jobs.delete('lighthouse')

    { 'changes' => { 'result' => 'success', 'outputs' => selections } }.merge(jobs)
  end

  def gate_script
    <<~JAVASCRIPT
      import { evaluate } from './scripts/ci/gate.mjs';
      console.log(JSON.stringify(evaluate(JSON.parse(process.argv[1]))));
    JAVASCRIPT
  end
end
