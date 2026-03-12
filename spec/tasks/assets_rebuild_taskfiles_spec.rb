# frozen_string_literal: true

require 'rails_helper'

RSpec.describe YAML do
  def task_config(path)
    YAML.safe_load(Rails.root.join(path).read)
  end

  def expected_commands(environment)
    [
      {
        'task' => ':internal:run',
        'vars' => {
          'ENVIRONMENT' => environment,
          'COMMAND' => 'find public/assets -mindepth 1 -maxdepth 1 -exec rm -rf {} +'
        }
      },
      {
        'task' => ':internal:run',
        'vars' => {
          'ENVIRONMENT' => environment,
          'COMMAND' => 'rails assets:precompile'
        }
      }
    ]
  end

  shared_examples 'assets rebuild taskfile' do |path, environment|
    it "defines #{environment}:assets-rebuild via internal:run" do
      task = task_config(path).dig('tasks', 'assets-rebuild')

      expect(task['cmds']).to eq(expected_commands(environment))
    end
  end

  include_examples 'assets rebuild taskfile', 'Taskfiles/dev.yml', 'dev'
  include_examples 'assets rebuild taskfile', 'Taskfiles/test.yml', 'test'
end
