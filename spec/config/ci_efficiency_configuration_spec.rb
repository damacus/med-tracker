# frozen_string_literal: true

require 'rails_helper'
require 'yaml'

module CiEfficiencyConfiguration
end

RSpec.describe CiEfficiencyConfiguration do
  let(:ci_workflow) { workflow('ci.yml') }
  let(:ci_jobs) { ci_workflow.fetch('jobs') }

  it 'cancels superseded pull request runs without cancelling main runs' do
    expect(ci_workflow.fetch('concurrency')).to eq(
      'group' => 'ci-${{ github.event.pull_request.number || github.ref }}',
      'cancel-in-progress' => "${{ github.event_name == 'pull_request' }}"
    )
  end

  it 'classifies application, Rust, and documentation changes' do
    changes = ci_jobs.fetch('changes')

    expect(changes.fetch('outputs')).to eq(
      'app' => '${{ steps.filter.outputs.app }}',
      'rust' => '${{ steps.filter.outputs.rust }}',
      'docs' => '${{ steps.filter.outputs.docs }}'
    )
    expect(changes.dig('steps', 1, 'uses')).to start_with('dorny/paths-filter@')
  end

  it 'runs application jobs only for application changes' do
    application_jobs = %w[
      scan_ruby scan_js lint test_non_system test_system
      production_image_medication_search mutation lighthouse
    ]

    application_jobs.each do |job_name|
      job = ci_jobs.fetch(job_name)

      expect(Array(job.fetch('needs'))).to include('changes')
      expect(job.fetch('if')).to include("needs.changes.outputs.app == 'true'")
    end
  end

  it 'runs Rust and documentation jobs only for their change groups' do
    expect(ci_jobs.fetch('client_tools')).to include(
      'needs' => 'changes',
      'if' => "${{ needs.changes.outputs.rust == 'true' }}"
    )
    expect(ci_jobs.fetch('docs_check')).to include(
      'needs' => 'changes',
      'if' => "${{ needs.changes.outputs.docs == 'true' }}"
    )
    expect(ci_jobs.fetch('docs_check').to_json).to include('zensical build --clean', 'git diff --check')
  end

  it 'provides one stable aggregate CI result' do
    gate = ci_jobs.fetch('ci_success')

    expect(gate.fetch('if')).to eq('${{ always() }}')
    expect(gate.fetch('needs')).to include('changes', 'docs_check', 'client_tools', 'lighthouse')
    expect(gate.to_json).to include("contains(needs.*.result, 'failure')")
    expect(gate.to_json).to include("contains(needs.*.result, 'cancelled')")
  end

  it 'uses the supported Intel macOS runner with a release timeout' do
    release_job = workflow('client-tools-release.yml').fetch('jobs').fetch('build')

    expect(release_job.fetch('timeout-minutes')).to eq(20)
    expect(release_job.dig('strategy', 'matrix', 'include')).to include(
      hash_including('artifact' => 'medtracker-client-tools-macos-x86_64', 'runner' => 'macos-15-intel')
    )
  end

  it 'publishes runtime changes and lets release-please own release images' do
    docker_workflow = workflow('docker-publish.yml')
    docker_events = events(docker_workflow)
    docker_push = docker_events.fetch('push')

    expect(docker_push).not_to have_key('tags')
    expect(docker_push.fetch('paths')).to include('app/**', 'Gemfile.lock', 'Dockerfile')
    expect(docker_workflow.fetch('concurrency')).to eq(
      'group' => 'container-${{ inputs.tag_name || github.ref }}',
      'cancel-in-progress' => "${{ inputs.tag_name == '' && github.ref == 'refs/heads/main' }}"
    )
    expect(workflow('release-please.yml').dig('jobs', 'docker', 'uses')).to eq(
      './.github/workflows/docker-publish.yml'
    )
  end

  def workflow(name)
    YAML.safe_load(Rails.root.join('.github/workflows', name).read, aliases: true)
  end

  def events(workflow_config)
    workflow_config.fetch('on') { workflow_config.fetch(true) }
  end
end
