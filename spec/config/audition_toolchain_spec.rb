# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Audition toolchain' do
  let(:taskfile) do
    YAML.safe_load(Rails.root.join('Taskfile.yml').read, aliases: true, permitted_classes: [Symbol])
  end
  let(:dockerfile) { Rails.root.join('Dockerfile').read }
  let(:audition_config) { YAML.safe_load(Rails.root.join('.audition.yml').read) }

  it 'installs the pinned scanner in the tools image outside the application bundle' do
    tools_stage = docker_stage('tools')

    expect(tools_stage).to include(
      'gem install audition --version 0.2.1 --no-document --install-dir /opt/audition'
    )
    expect(tools_stage).to include('GEM_PATH="/opt/audition:/usr/local/bundle:/usr/local/lib/ruby/gems/4.0.0"')
    expect(tools_stage).not_to include('bundle add audition')
  end

  it 'defines reviewed exclusions and a deliberate baseline-writing command' do
    expect(audition_config).to include(
      'fail_on' => 'error',
      'timeout' => 60,
      'exclude' => %w[db/schema.rb db/migrate/** spec/** tmp/** vendor/** node_modules/** public/assets/**]
    )
    expect(audition_task('baseline').dig('cmds', 0, 'vars', 'COMMAND'))
      .to eq('audition . --static-only --write-baseline')
  end

  it 'runs Audition through the tools image without Bundler for every scan mode' do
    expected_commands = {
      'static' => 'audition . --static-only --no-baseline',
      'dependencies' => 'audition Gemfile.lock --static-only',
      'dynamic' => 'audition . --dynamic-only',
      'target' => 'audition {{ .target }} --static-only --no-baseline',
      'fix-preview' => 'audition {{ .target }} --no-baseline --fix-unsafe --dry-run',
      'ci' => 'audition . --static-only --format github'
    }

    expected_commands.each do |name, command|
      task = audition_task(name)

      expect(task.dig('cmds', 0, 'task')).to eq('internal:run')
      expect(task.dig('cmds', 0, 'vars')).to include(
        'ENVIRONMENT' => 'test',
        'SERVICE' => 'tools-test',
        'DOCKER_RUN_ARGS' => '--build',
        'COMMAND' => command
      )
      expect(command).not_to include('bundle exec')
    end
  end

  it 'keeps mutation opt-in by exposing only a dry-run fix preview' do
    command = audition_task('fix-preview').dig('cmds', 0, 'vars', 'COMMAND')

    expect(command).to include('--fix-unsafe', '--dry-run')
  end

  def audition_task(name)
    taskfile.dig('tasks', "audition:#{name}") || {}
  end

  def docker_stage(name)
    lines = dockerfile.lines
    start_index = lines.find_index { |line| line.match?(/^FROM .+ AS #{Regexp.escape(name)}$/) }
    raise KeyError, "Docker stage not found: #{name}" unless start_index

    end_index = lines[(start_index + 1)..].find_index { |line| line.start_with?('FROM ') }
    end_index ? lines[start_index, end_index + 1].join : lines[start_index..].join
  end
end
