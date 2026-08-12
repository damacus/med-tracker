#!/usr/bin/env ruby

require 'active_support/inflector'
require 'bundler/setup'
require 'fileutils'
require 'open3'
require 'optparse'
require 'pathname'
require 'tmpdir'
require 'yaml'

class RubyUiComparison
  def initialize(arguments, root: Pathname.pwd, stdout: $stdout, stderr: $stderr)
    @arguments = arguments
    @root = root.expand_path
    @stdout = stdout
    @stderr = stderr
  end

  def run
    output, components = parse_options
    return 1 unless components
    return 1 unless load_generator
    return 1 unless validate_components(components)
    return 1 unless safe_output?(output)

    workspace = create_workspace
    generated_root = generate(workspace, components)
    return 1 unless generated_root

    destination = persist_generated_files(generated_root, output, components)
    report(destination, components)
    0
  ensure
    FileUtils.remove_entry(workspace) if workspace&.exist?
  end

  private

  def parse_options
    options = {}
    parser = OptionParser.new do |opts|
      opts.on('--output DIRECTORY', 'Keep generated files in a new external directory') do |directory|
        options[:output] = Pathname.new(directory).expand_path
      end
    end
    components = parser.parse(@arguments).uniq
    if components.empty?
      @stderr.puts 'Pass one or more RubyUI component names.'
      return [nil, nil]
    end

    [options[:output], components]
  rescue OptionParser::ParseError => error
    @stderr.puts error.message
    [nil, nil]
  end

  def load_generator
    spec = Gem.loaded_specs['ruby_ui']
    unless spec
      @stderr.puts 'Could not load the Bundler-activated RubyUI dependency.'
      return false
    end

    @generator_version = spec.version.to_s
    @generator_source = Pathname.new(spec.full_gem_path)
    @dependencies = YAML.safe_load(@generator_source.join('lib/generators/ruby_ui/dependencies.yml').read)
    return true if @generator_version == locked_version

    @stderr.puts "Bundler-activated RubyUI version #{@generator_version} does not match Gemfile.lock #{locked_version}."
    false
  end

  def locked_version
    @root.join('Gemfile.lock').read[/ruby_ui \(([^)]+)\)/, 1]
  end

  def validate_components(components)
    unknown = components.reject { canonical_component?(it) }
    return true if unknown.empty?

    @stderr.puts "Unknown RubyUI component: #{unknown.join(', ')}"
    false
  end

  def canonical_component?(component)
    component.match?(/\A[A-Za-z][A-Za-z0-9_]*\z/) && component_source(component).directory?
  end

  def external_temp_parent
    %w[/private/tmp /tmp].filter_map do |candidate|
      path = Pathname.new(candidate)
      path.realpath if path.directory? && !within?(path.realpath, @root.realpath)
    rescue Errno::ENOENT, Errno::EACCES
      nil
    end.first || raise('No external temporary directory is available')
  end

  def create_workspace
    Pathname.new(Dir.mktmpdir('med-tracker-ruby-ui-generator-', external_temp_parent))
  end

  def safe_output?(output)
    return true unless output
    return unsafe_output(output, 'parent directory must already exist') unless output.dirname.directory?
    return unsafe_output(output, 'must not already exist') if output.exist? || output.symlink?
    return unsafe_output(output, 'must not traverse symbolic links') if symlink_in_path?(output.dirname)

    canonical_output = output.dirname.realpath.join(output.basename)
    return unsafe_output(output, 'must be outside the application checkout') if within?(canonical_output, @root.realpath)

    @output = canonical_output
    true
  rescue Errno::ENOENT, Errno::EACCES => error
    unsafe_output(output, "is not a usable isolated destination: #{error.message}")
  end

  def symlink_in_path?(path)
    current = Pathname.new(File::SEPARATOR)
    path.each_filename do |name|
      current = current.join(name)
      return true if current.symlink?
    end
    false
  end

  def within?(path, root)
    path == root || path.to_s.start_with?("#{root}/")
  end

  def unsafe_output(output, reason)
    @stderr.puts "Output directory #{output} #{reason}."
    false
  end

  def generate(workspace, components)
    copy_application(workspace)
    install_read_only_generator(workspace)
    command = [
      Gem.bin_path('bundler', 'bundle'), 'exec', workspace.join('bin/rails').to_s, 'generate',
      'ruby_ui:comparison_component', *components, '--force'
    ]
    environment = {
      'BUNDLE_GEMFILE' => workspace.join('Gemfile').to_s,
      'RUBYOPT' => '-r./.ruby_ui_comparison_generator'
    }
    _output, error, status = Open3.capture3(environment, *command, chdir: workspace)
    return workspace if status.success?

    @stderr.puts "RubyUI generator failed: #{error.strip}"
    nil
  end

  def copy_application(workspace)
    %w[app bin config lib Gemfile Gemfile.lock Rakefile].each do |entry|
      FileUtils.cp_r(@root.join(entry), workspace)
    end
  end

  def install_read_only_generator(workspace)
    path = workspace.join('.ruby_ui_comparison_generator.rb')
    path.write(<<~RUBY)
      require 'rails/generators'
      require 'generators/ruby_ui/component_generator'

      module RubyUI
        module Generators
          class ComparisonComponentGenerator < ComponentGenerator
            namespace 'ruby_ui:comparison_component'
            source_root ComponentGenerator.source_root

            private

            def install_dependencies(*) = nil
            def update_stimulus_manifest = nil
          end
        end
      end
    RUBY
    path
  end

  def persist_generated_files(workspace, output, components)
    return workspace unless output

    Dir.mkdir(@output)
    expected_paths(components).each_key do |path|
      source = workspace.join(path)
      next unless source.file?

      destination = @output.join(path)
      destination.dirname.mkpath
      FileUtils.cp(source, destination)
    end
    @output
  end

  def report(generated_root, components)
    generated = generated_files(generated_root, components)
    local = local_files(components)
    common = generated.keys & local.keys
    unchanged = common.select { FileUtils.compare_file(generated[it], local[it]) }
    changed = common - unchanged
    missing = generated.keys - local.keys
    local_only = local.keys - generated.keys

    @stdout.puts "RubyUI version: #{@generator_version}"
    @stdout.puts 'Generator: RubyUI::Generators::ComponentGenerator (read-only dependency hooks)'
    @stdout.puts "Generator source: #{@generator_source}"
    @stdout.puts "Requested components: #{components.join(', ')}"
    components.each { print_dependencies(it) }
    print_files('Unchanged files', unchanged)
    print_files('Changed files', changed)
    print_files('Missing locally', missing)
    print_files('Local-only files', local_only)
  end

  def generated_files(root, components)
    expected_paths(components).to_h do |path, _source|
      [path, root.join(path)]
    end.select { |_path, file| file.file? }
  end

  def local_files(components)
    components.each_with_object({}) do |component, files|
      family = family_name(component)
      directory = @root.join('app/components/ruby_ui', family)
      directory.glob('*.rb').each { files[it.relative_path_from(@root).to_s] = it } if directory.directory?
      expected_paths([component]).each_key do |path|
        file = @root.join(path)
        files[path] = file if path.end_with?('.js') && file.file?
      end
      local_controller_files(family).each do |file|
        files[file.relative_path_from(@root).to_s] = file
      end
    end
  end

  def local_controller_files(family)
    root = @root.join('app/javascript/controllers/ruby_ui')
    return [] unless root.directory?

    root.glob("#{family}{,_*}.js").select(&:file?)
  end

  def expected_paths(components)
    components.each_with_object({}) do |component, files|
      family = family_name(component)
      component_source(component).glob('*.rb').reject { it.basename.to_s.end_with?('_docs.rb') }.each do |source|
        files["app/components/ruby_ui/#{family}/#{source.basename}"] = source
      end
      component_source(component).glob('*.js').each do |source|
        files["app/javascript/controllers/ruby_ui/#{source.basename}"] = source
      end
    end
  end

  def component_source(component)
    @generator_source.join('lib/ruby_ui', family_name(component))
  end

  def family_name(component)
    component.underscore
  end

  def print_dependencies(component)
    dependencies = @dependencies.fetch(family_name(component), {})
    fields = {
      components: dependencies.fetch('components', []),
      gems: dependencies.fetch('gems', []),
      javascript: dependencies.fetch('js_packages', [])
    }
    summary = fields.map { |name, values| "#{name}=#{values.any? ? values.join(', ') : 'none'}" }.join('; ')
    @stdout.puts "Declared dependencies (#{component}): #{summary}"
  end

  def print_files(label, files)
    @stdout.puts "#{label}: #{files.empty? ? 'none' : files.sort.join(', ')}"
  end
end

exit RubyUiComparison.new(ARGV).run if $PROGRAM_NAME == __FILE__
