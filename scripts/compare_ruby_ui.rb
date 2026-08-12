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
    return 1 unless create_output(output, options)

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
      opts.on('--from-task-environment', 'Read task inputs from the environment') do
        options[:all] = ENV['RUBY_UI_ALL'] == '1'
        options[:output] = ENV['RUBY_UI_OUTPUT'] unless ENV['RUBY_UI_OUTPUT'].to_s.empty?
        options[:components] = ENV.fetch('RUBY_UI_COMPONENTS', '').split
      end
    end
    components = parser.parse(@arguments)
    [options, options.fetch(:components, components)]
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
    return true unless output_option?

    return unsafe_output('parent directory must already exist') unless output.dirname.directory?
    return unsafe_output('must not traverse symbolic links') if symlink_in_path?(output.dirname)

    canonical_root = @root.realpath
    canonical_parent = output.dirname.realpath
    canonical_output = canonical_parent.join(output.basename)
    return unsafe_output('must be outside the application checkout') if within?(canonical_output, canonical_root)
    return unsafe_output('must not already exist') if output.exist? || output.symlink?

    @output = canonical_output
    true
  rescue Errno::ENOENT, Errno::EACCES => error
    unsafe_output("is not a usable isolated destination: #{error.message}")
  end

  def create_output(output, options)
    return true unless options[:output]

    Dir.mkdir(@output || output)
    created = @output || output
    return true if created.directory? && !created.symlink? && created.realpath == created

    unsafe_output('must resolve to the newly created isolated directory')
  rescue Errno::EEXIST
    unsafe_output('must not already exist')
  rescue SystemCallError => error
    unsafe_output("could not be created: #{error.message}")
  end

  def output_option?
    !@arguments.empty? && @arguments.each_cons(2).any? { |flag, _value| flag == '--output' } || ENV['RUBY_UI_OUTPUT'].to_s != '' && @arguments.include?('--from-task-environment')
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

  def unsafe_output(reason)
    @stderr.puts "Output directory #{requested_output} #{reason}."
    false
  end

  def requested_output
    @output || @arguments.each_cons(2).find { |flag, _value| flag == '--output' }&.last || ENV['RUBY_UI_OUTPUT']
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
    install_recording_shims(output)
    return [nil, [], []] unless generator_spec(output)

    generator = options[:all] ? 'ruby_ui:component:all' : 'ruby_ui:component'
    registration_before = registration_files(output)
    command = [
      bundle_command, 'exec', 'bin/rails', 'generate', generator, *components, '--force',
      '--destination-root', output.to_s
    ]
    stdout, error, status = capture_in_output_bundle(output, *command)
    if status.success?
      side_effects = recorded_dependency_changes(output) + changed_registration_files(output, registration_before)
      return [output, generated_paths(stdout), side_effects.uniq.sort]
    end

    @stderr.puts "RubyUI generator failed: #{error.strip}"
    [nil, [], []]
  end

  def copy_application(output)
    output.mkpath
    %w[app bin config lib Gemfile Gemfile.lock Rakefile].each do |entry|
      FileUtils.cp_r(@root.join(entry), output)
    end
  end

  def install_recording_shims(output)
    record = output.join('.ruby-ui-comparison-side-effects')
    record.write('')
    install_bundle_shim(output, record)
    install_importmap_shim(output, record)
  end

  def install_bundle_shim(output, record)
    output.join('bin/bundle').write(<<~RUBY)
      #!/usr/bin/env ruby
      record = #{record.to_s.inspect}
      case ARGV.first
      when 'show'
        ENV['BUNDLE_GEMFILE'] = #{output.join('Gemfile').to_s.inspect}
        exec #{bundle_command.inspect}, *ARGV
      when 'add'
        File.open(record, 'a') { it.puts("gem \#{ARGV.drop(1).join(' ')}") }
        exit 0
      else
        ENV['BUNDLE_GEMFILE'] = #{output.join('Gemfile').to_s.inspect}
        exec #{bundle_command.inspect}, *ARGV
      end
    RUBY
    FileUtils.chmod('+x', output.join('bin/bundle'))
  end

  def install_importmap_shim(output, record)
    original = output.join('bin/importmap.ruby-ui-comparison-original')
    FileUtils.mv(output.join('bin/importmap'), original)
    output.join('bin/importmap').write(<<~RUBY)
      #!/usr/bin/env ruby
      if ARGV.first == 'pin'
        package = ARGV[1]
        source = File.read(#{output.join('config/importmap.rb').to_s.inspect})
        unless source.match?(/^\\s*pin\\s+['\"]\#{Regexp.escape(package)}['\"]/)
          File.open(#{record.to_s.inspect}, 'a') { it.puts("javascript \#{ARGV.drop(1).join(' ')}") }
        end
        exit 0
      end

      exec #{original.to_s.inspect}, *ARGV
    RUBY
    FileUtils.chmod('+x', output.join('bin/importmap'))
  end

  def report(generated_root, generated_paths, components, options, registrations)
    generated_files = ruby_ui_files(generated_root)
    local_files = ruby_ui_files(@root)
    generated = generated_files.slice(*generated_paths)
    local = options[:all] ? local_files : local_files.slice(*component_scope(generated_paths, generated_files, local_files, components))
    upstream_only = generated.keys - local.keys
    local_only = local.keys - generated.keys
    changed = (generated.keys & local.keys).select { |path| !FileUtils.compare_file(generated[path], local[path]) }

    @stdout.puts "RubyUI version: #{@generator_version}"
    @stdout.puts "Generator: bin/rails generate ruby_ui:component#{':all' if options[:all]}"
    @stdout.puts "Generator source: #{@generator_source}"
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

  def generator_spec(output)
    command = [
      bundle_command, 'exec', 'ruby', '-e',
      "spec = Gem::Specification.find_by_name('ruby_ui'); puts spec.version; puts spec.full_gem_path"
    ]
    stdout, error, status = capture_in_output_bundle(output, *command)
    unless status.success?
      @stderr.puts "Could not load the Bundler-activated RubyUI generator: #{error.strip}"
      return false
    end

    @generator_version, @generator_source = stdout.lines(chomp: true)
    return true if @generator_version == locked_version

    @stderr.puts "Bundler-activated RubyUI version #{@generator_version.inspect} does not match Gemfile.lock #{locked_version.inspect}."
    false
  end

  def capture_in_output_bundle(output, *command)
    environment = {
      'BUNDLE_GEMFILE' => output.join('Gemfile').to_s,
      'PATH' => [output.join('bin'), ENV.fetch('PATH')].join(File::PATH_SEPARATOR)
    }
    Open3.capture3(environment, *command, chdir: output)
  end

  def bundle_command
    Gem.bin_path('bundler', 'bundle')
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

  def recorded_dependency_changes(root)
    root.join('.ruby-ui-comparison-side-effects').read.lines(chomp: true)
  end

  def component_scope(generated_paths, generated_files, local_files, components)
    component_directories = generated_paths.filter_map { |path| path[%r{\A(app/components/ruby_ui/[^/]+/)}] }
    component_directories.concat(components.map { |component| "app/components/ruby_ui/#{underscore(component)}/" })
    controller_prefixes = generated_paths.filter_map do |path|
      path[%r{\A(app/javascript/controllers/ruby_ui/[^/]+?)(?:_controller\.js)?\z}, 1]
    end
    candidates = generated_files.keys | local_files.keys

    candidates.select do |path|
      component_directories.any? { path.start_with?(it) } || controller_prefixes.any? { path.start_with?(it) }
    end
  end

  def underscore(component)
    component.gsub(/([a-z\d])([A-Z])/, '\\1_\\2').downcase
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
