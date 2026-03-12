# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'assets rebuild taskfiles' do
  def task_config(path)
    YAML.safe_load(Rails.root.join(path).read)
  end

  it 'defines dev:assets-rebuild via internal:run' do
    task = task_config('Taskfiles/dev.yml').dig('tasks', 'assets-rebuild')

    expect(task['cmds']).to eq([
                                 {
                                   'task' => ':internal:run',
                                   'vars' => {
                                     'ENVIRONMENT' => 'dev',
                                     'COMMAND' => 'find public/assets -mindepth 1 -maxdepth 1 -exec rm -rf {} +'
                                   }
                                 },
                                 {
                                   'task' => ':internal:run',
                                   'vars' => {
                                     'ENVIRONMENT' => 'dev',
                                     'COMMAND' => 'rails assets:precompile'
                                   }
                                 }
                               ])
  end

  it 'defines test:assets-rebuild via internal:run' do
    task = task_config('Taskfiles/test.yml').dig('tasks', 'assets-rebuild')

    expect(task['cmds']).to eq([
                                 {
                                   'task' => ':internal:run',
                                   'vars' => {
                                     'ENVIRONMENT' => 'test',
                                     'COMMAND' => 'find public/assets -mindepth 1 -maxdepth 1 -exec rm -rf {} +'
                                   }
                                 },
                                 {
                                   'task' => ':internal:run',
                                   'vars' => {
                                     'ENVIRONMENT' => 'test',
                                     'COMMAND' => 'rails assets:precompile'
                                   }
                                 }
                               ])
  end
end
