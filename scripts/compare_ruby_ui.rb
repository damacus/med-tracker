#!/usr/bin/env ruby

require 'fileutils'
require 'open3'
require 'optparse'
require 'pathname'
require 'tmpdir'

class RubyUiComparison
  def initialize(arguments, root: Pathname.pwd, stdout: $stdout, stderr: $stderr)
    @arguments = arguments
    @root = root
    @stdout = stdout
    @stderr = stderr
  end

  def run
    options, components = parse_options
    return 1 unless valid_request?(options, components)

    output = isolated_output(options)
    return 1 unless safe_output?(output)
    return 1 if dirty_checkout? && !options[:output]

    generated_root, generated_paths, registrations = generate(output, options, components)
    return 1 unless generated_root

    report(generated_root, generated_paths, components, options, registrations)
    0
  ensure
    FileUtils.remove_entry(output) if output && !options&.fetch(:output, nil)
  end

  private

  def parse_options
    options = { all: false }
    parser = OptionParser.new do |opts|
      opts.on('--all', 'Compare every RubyUI component') { options[:all] = true }
      opts.on('--output DIRECTORY', 'Use an isolated output directory') { |directory| options[:output] = directory }
    end
    [options, parser.parse(@arguments)]
  rescue OptionParser::ParseError => error
    @stderr.puts error.message
    [options, []]
  end

  def valid_request?(options, components)
    return true if options[:all] || components.any?

    @stderr.puts 'Pass one or more RubyUI component names or --all.'
    false
  end

  def isolated_output(options)
    return Pathname.new(options[:output]).expand_path if options[:output]

    Pathname.new(Dir.mktmpdir('med-tracker-ruby-ui-'))
  end

  def safe_output?(output)
    return true unless output.to_s.start_with?(@root.to_s)

    @stderr.puts "Output directory #{output} must be outside the application checkout."
    false
  end

  def dirty_checkout?
    output, _error, status = Open3.capture3('git', 'status', '--porcelain', chdir: @root)
    return false unless status.success?

    return false if output.empty?

    @stderr.puts 'Checkout has uncommitted changes. Pass --output DIRECTORY outside the application checkout.'
    true
  end

  def generate(output, options, components)
    copy_application(output)
    generator = options[:all] ? 'ruby_ui:component:all' : 'ruby_ui:component'
    registration_before = registration_files(output)
    command = ['bin/rails', 'generate', generator, *components, '--force']
    stdout, error, status = Open3.capture3(*command, chdir: output)
    return [output, generated_paths(stdout), changed_registration_files(output, registration_before)] if status.success?

    @stderr.puts "RubyUI generator failed: #{error.strip}"
    [nil, [], []]
  end

  def copy_application(output)
    output.mkpath
    %w[app bin config lib Gemfile Gemfile.lock Rakefile].each do |entry|
      FileUtils.cp_r(@root.join(entry), output)
    end
  end

  def report(generated_root, generated_paths, components, options, registrations)
    generated = ruby_ui_files(generated_root).slice(*generated_paths)
    local = ruby_ui_files(@root)
    upstream_only = generated.keys - local.keys
    local_only = local.keys - generated.keys
    changed = (generated.keys & local.keys).select { |path| !FileUtils.compare_file(generated[path], local[path]) }

    @stdout.puts "RubyUI version: #{locked_version}"
    @stdout.puts "Generator: bin/rails generate ruby_ui:component#{':all' if options[:all]}"
    @stdout.puts "Generator source: #{Gem::Specification.find_by_name('ruby_ui').full_gem_path}"
    @stdout.puts "Requested components: #{options[:all] ? 'all' : components.join(', ')}"
    print_files('Generated files', generated.keys)
    print_files('Local-only files', local_only)
    print_files('Upstream-only files', upstream_only)
    print_files('Changed files', changed)
    print_files('Dependency or controller-registration changes', registrations)
  end

  def locked_version
    @root.join('Gemfile.lock').read[/ruby_ui \(([^)]+)\)/, 1]
  end

  def ruby_ui_files(root)
    component_root = root.join('app/components/ruby_ui')
    controller_root = root.join('app/javascript/controllers/ruby_ui')
    files_in(component_root, root).merge(files_in(controller_root, root))
  end

  def files_in(directory, root)
    return {} unless directory.exist?

    directory.glob('**/*').select(&:file?).to_h { |path| [path.relative_path_from(root).to_s, path] }
  end

  def registration_files(root)
    %w[Gemfile package.json config/importmap.rb app/javascript/controllers/index.js].to_h do |path|
      candidate = root.join(path)
      [path, candidate.exist? ? candidate.binread : nil]
    end
  end

  def changed_registration_files(root, before)
    registration_files(root).filter_map { |path, content| path if before[path] != content }
  end

  def generated_paths(output)
    output.each_line.filter_map do |line|
      line.match(/\A\s*(?:create|force|identical)\s+(app\/(?:components\/ruby_ui|javascript\/controllers\/ruby_ui)\/\S+)/)&.captures&.first
    end.uniq.sort
  end

  def print_files(label, files)
    @stdout.puts "#{label}: #{files.empty? ? 'none' : files.join(', ')}"
  end
end

exit RubyUiComparison.new(ARGV).run if $PROGRAM_NAME == __FILE__
