require 'simplecov'
require 'json'

abort 'COVERAGE=true is required for SimpleCov collation' unless ENV['COVERAGE'] == 'true'

result_files = ARGV
abort 'Exactly two SimpleCov result files are required' unless result_files.length == 2
expected_names = %w[rspec-non-system-shard-1 rspec-non-system-shard-2]
result_names = result_files.map do |path|
  abort "SimpleCov result file is missing: #{path}" unless File.file?(path)

  begin
    result = JSON.parse(File.read(path))
  rescue JSON::ParserError => e
    abort "SimpleCov result file is malformed: #{path} (#{e.message})"
  end
  valid_result = result.is_a?(Hash) && result.any? && result.values.first.is_a?(Hash) && result.values.first.any?
  abort "SimpleCov result file is empty: #{path}" unless valid_result
  abort "SimpleCov result file must contain one command: #{path}" unless result.size == 1

  name = result.keys.first
  abort "Unexpected SimpleCov command name: #{name}" unless expected_names.include?(name)
  name
end
abort 'SimpleCov result files must have distinct command names' unless result_names.uniq.length == 2

ENV.delete('SIMPLECOV_SHARD')
load File.expand_path('../../.simplecov', __dir__)
SimpleCov.coverage_path ENV.fetch('SIMPLECOV_COVERAGE_DIR') if ENV['SIMPLECOV_COVERAGE_DIR']
SimpleCov.collate(result_files, 'rails')
