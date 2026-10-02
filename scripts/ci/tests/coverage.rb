require 'fileutils'
require 'open3'
require 'tmpdir'

repository = File.expand_path('../../..', __dir__)
collator = File.join(repository, 'scripts/ci/collate_simplecov.rb')

Dir.mktmpdir('coverage-check') do |directory|
  source = File.join(directory, 'app/controllers/api/example.rb')
  FileUtils.mkdir_p(File.dirname(source))
  File.write(source, "if ENV.fetch('BRANCH') == 'yes'\n  :yes\nelse\n  :no\nend\n")
  environment = { 'BUNDLE_GEMFILE' => File.join(repository, 'Gemfile'), 'COVERAGE' => 'true' }

  reports = %w[yes no].each_with_index.map do |branch, index|
    output_path = File.join(directory, "shard-#{index + 1}")
    script = <<~RUBY
      require 'simplecov'
      SimpleCov.start do
        root #{directory.inspect}
        coverage_dir #{output_path.inspect}
        command_name 'rspec-non-system-shard-#{index + 1}'
        enable_coverage :branch
      end
      load #{source.inspect}
    RUBY
    output, status = Open3.capture2e(environment.merge('BRANCH' => branch),
                                   'bundle', 'exec', 'ruby', '-e', script, chdir: directory)
    abort output unless status.success?
    File.join(output_path, '.resultset.json')
  end

  run = lambda do |paths|
    Open3.capture2e(environment.merge('SIMPLECOV_COVERAGE_DIR' => File.join(directory, 'merged')),
                   'bundle', 'exec', 'ruby', collator, *paths, chdir: directory)
  end

  output, status = run.call(reports)
  abort "Complementary coverage failed:\n#{output}" unless status.success?

  [[reports.first], [reports.first, reports.first]].each do |paths|
    output, status = run.call(paths)
    abort "Incomplete or duplicate coverage was accepted:\n#{output}" if status.success?
  end

  require 'json'
  first = JSON.parse(File.read(reports.first)).values.first
  File.write(reports.last, JSON.generate('rspec-non-system-shard-2' => first))
  FileUtils.rm_rf(File.join(directory, 'merged'))
  output, status = run.call(reports)
  abort 'Incomplete coverage passed' if status.success?
  %w[Line\ coverage Branch\ coverage API\ branch\ coverage].each do |message|
    abort "Coverage gate did not report #{message}:\n#{output}" unless output.include?(message)
  end
end

puts 'Coverage collation passes complete results and rejects missing, duplicate and below-threshold results.'
