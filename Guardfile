# frozen_string_literal: true

# Guard configuration for MedTracker
# Runs RSpec tests in Docker via task commands
# More info at https://github.com/guard/guard#readme

# Watch only relevant directories to reduce noise
directories %w[app spec config lib]

# Guard notification settings for readable output
notification :terminal_notifier, app_name: 'MedTracker' if `uname`.strip == 'Darwin'
notification :tmux, display_message: true if ENV['TMUX']

# Configure Guard with clear, actionable output
guard :rspec,
      cmd: 'task test TEST_FILE=',
      all_on_start: false,
      all_after_pass: false,
      failed_mode: :focus,
      notification: true do
  require 'guard/rspec/dsl'
  dsl = Guard::RSpec::Dsl.new(self)

  # Feel free to open issues for suggestions and improvements

  # RSpec files
  rspec = dsl.rspec
  watch(rspec.spec_helper) { rspec.spec_dir }
  watch(rspec.spec_support) { rspec.spec_dir }
  watch(rspec.spec_files)

  # Ruby files
  ruby = dsl.ruby
  dsl.watch_spec_files_for(ruby.lib_files)

  # Rails files - watch Phlex components (.rb) instead of ERB
  rails = dsl.rails(view_extensions: %w[rb])
  dsl.watch_spec_files_for(rails.app_files)

  # Watch Phlex view components
  watch(%r{^app/views/(.+)\.rb$}) { |m| "spec/components/#{m[1]}_spec.rb" }
  watch(%r{^app/components/(.+)\.rb$}) { |m| "spec/components/#{m[1]}_spec.rb" }

  # Watch controllers and run corresponding specs
  watch(%r{^app/controllers/(.+)_controller\.rb$}) do |m|
    [
      "spec/requests/#{m[1]}_spec.rb",
      "spec/features/#{m[1]}_spec.rb"
    ]
  end

  # Watch models and run model specs
  watch(%r{^app/models/(.+)\.rb$}) { |m| "spec/models/#{m[1]}_spec.rb" }

  # Watch policies and run policy specs
  watch(%r{^app/policies/(.+)\.rb$}) { |m| "spec/policies/#{m[1]}_spec.rb" }

  # Rails config changes - run all specs
  watch('config/routes.rb') { rspec.spec_dir }
  watch(%r{^config/initializers/.+\.rb$}) { rspec.spec_dir }

  # Run all specs when spec_helper or rails_helper changes
  watch('spec/spec_helper.rb') { rspec.spec_dir }
  watch('spec/rails_helper.rb') { rspec.spec_dir }
end
