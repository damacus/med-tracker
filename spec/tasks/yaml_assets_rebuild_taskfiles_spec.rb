# frozen_string_literal: true

require 'rails_helper'

CLEAR_ASSETS_CMD = 'find public/assets -mindepth 1 -maxdepth 1 -exec rm -rf {} +'

RSpec.describe YAML do
  def task_cmds(path)
    YAML.safe_load(Rails.root.join(path).read).dig('tasks', 'assets-rebuild', 'cmds')
  end

  def internal_run(environment, command)
    { 'task' => ':internal:run', 'vars' => { 'ENVIRONMENT' => environment, 'COMMAND' => command } }
  end

  it 'defines dev:assets-rebuild via internal:run' do
    expect(task_cmds('Taskfiles/dev.yml')).to eq([
                                                   internal_run('dev', CLEAR_ASSETS_CMD),
                                                   internal_run('dev', 'rails assets:precompile')
                                                 ])
  end

  it 'defines test:assets-rebuild via internal:run' do
    expect(task_cmds('Taskfiles/test.yml')).to eq([
                                                    internal_run('test', CLEAR_ASSETS_CMD),
                                                    internal_run('test', 'rails assets:precompile')
                                                  ])
  end
end
