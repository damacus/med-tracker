#!/usr/bin/env ruby
# frozen_string_literal: true

# Hook script to check if Gemfile was modified and run bundle install if needed.
# Receives JSON input from Cascade hook with file_path information.

require 'json'

def main
  input_data = JSON.parse($stdin.read)
  file_path = input_data.dig('tool_info', 'file_path') || ''

  # Check if the modified file is Gemfile
  if file_path.include?('Gemfile') || file_path.end_with?('/Gemfile')
    puts '🔧 Gemfile modified, running bundle install...'

    # Run bundle install
    result = system('bundle install')

    if result
      puts '✅ Bundle install completed successfully'
    else
      puts '❌ Bundle install failed'
      exit 1
    end
  end
rescue StandardError => e
  puts "Error in check_gemfile.rb: #{e.message}"
  exit 1
end

main
