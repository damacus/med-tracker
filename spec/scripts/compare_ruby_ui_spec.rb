# frozen_string_literal: true

require 'rails_helper'
require 'open3'
require 'tmpdir'
require 'stringio'
require Rails.root.join('scripts/compare_ruby_ui')

# rubocop:disable RSpec/DescribeClass
RSpec.describe 'RubyUI comparison command' do
  # rubocop:enable RSpec/DescribeClass
  it 'requires explicit components unless all components are explicitly requested' do
    _output, error, status = compare

    expect(status).not_to be_success
    expect(error).to include('Pass one or more RubyUI component names or --all.')
  end

  it 'refuses an output directory inside the application checkout' do
    _output, error, status = compare('--output', Rails.root.to_s, 'Button')

    expect(status).not_to be_success
    expect(error).to include('must be outside the application checkout')
  end

  it 'refuses a dirty checkout without explicit isolated output' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      root.mkpath
      File.write(root.join('Gemfile'), "source 'https://rubygems.org'\n")
      run_git(root, 'init')
      run_git(root, 'add', 'Gemfile')
      run_git(root, '-c', 'user.name=Test User', '-c', 'user.email=test@example.com', 'commit', '-m', 'initial')
      File.write(root.join('Gemfile'), "source 'https://rubygems.org'\n# dirty\n")

      error = StringIO.new
      expect(RubyUiComparison.new(['Button'], root:, stderr: error).run).to eq(1)
      expect(error.string).to include('Checkout has uncommitted changes')
    end
  end

  it 'permits a newly created isolated output on a dirty checkout' do
    Dir.mktmpdir do |directory|
      generated = Pathname.new(directory).join('generated')
      output, error, status = compare('--output', generated.to_s, 'Button')

      expect_isolated_comparison_output(output)
      expect_isolated_comparison_state(error, status, generated)
    end
  end

  it 'refuses an existing output directory before copying application files' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('existing')
      output.mkpath
      marker = output.join('keep.txt')
      File.write(marker, 'do not overwrite')

      _stdout, error, status = compare('--output', output.to_s, 'Button')

      expect(status).not_to be_success
      expect(error).to include('must not already exist')
      expect(marker.read).to eq('do not overwrite')
    end
  end

  it 'refuses a symlinked output path without following it' do
    Dir.mktmpdir do |directory|
      destination = Pathname.new(directory).join('destination')
      destination.mkpath
      marker = destination.join('keep.txt')
      File.write(marker, 'do not overwrite')
      output = Pathname.new(directory).join('output')
      File.symlink(destination, output)

      _stdout, error, status = compare('--output', output.to_s, 'Button')

      expect(status).not_to be_success
      expect(error).to include('must not already exist')
      expect(marker.read).to eq('do not overwrite')
    end
  end

  it 'refuses an output path that traverses a symbolic link' do
    Dir.mktmpdir do |directory|
      destination = Pathname.new(directory).join('destination')
      destination.mkpath
      output_parent = Pathname.new(directory).join('output-parent')
      File.symlink(destination, output_parent)

      _stdout, error, status = compare('--output', output_parent.join('generated').to_s, 'Button')

      expect(status).not_to be_success
      expect(error).to include('must not traverse symbolic links')
      expect(destination.join('generated')).not_to exist
    end
  end

  it 'reports an unchanged component when its generated output matches local source' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      copy_application(root)
      generated = Pathname.new(directory).join('generated-first')
      first_output = StringIO.new

      expect(RubyUiComparison.new(['--output', generated.to_s, 'Button'], root:, stdout: first_output).run).to eq(0)
      copy_generated_button(generated, root)

      second_output = StringIO.new
      second_generated = Pathname.new(directory).join('generated-second')
      comparison = RubyUiComparison.new(['--output', second_generated.to_s, 'Button'], root:, stdout: second_output)
      expect(comparison.run).to eq(0)
      expect(second_output.string).to include('Changed files: none')
    end
  end

  it 'reports a generated Stimulus controller difference' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      generated = Pathname.new(directory).join('generated-first')
      copy_application(root)

      expect(RubyUiComparison.new(['--output', generated.to_s, 'Dialog'], root:, stdout: StringIO.new).run).to eq(0)
      copy_generated_dialog(generated, root)
      File.write(root.join('app/javascript/controllers/ruby_ui/dialog_controller.js'), 'local change')

      output = StringIO.new
      next_generated = Pathname.new(directory).join('generated-second')
      expect(RubyUiComparison.new(['--output', next_generated.to_s, 'Dialog'], root:, stdout: output).run).to eq(0)
      expect(output.string).to include('Changed files: app/javascript/controllers/ruby_ui/dialog_controller.js')
    end
  end

  it 'reports a missing component without changing tracked application files' do
    before = tracked_application_diff

    Dir.mktmpdir do |directory|
      generated = Pathname.new(directory).join('generated')
      _output, error, status = compare('--output', generated.to_s, 'NotARealRubyUiComponent')

      expect(status).not_to be_success
      expect(error).to include('RubyUI generator failed')
    end
    expect(tracked_application_diff).to eq(before)
  end

  it 'compares all components only through the public --all option without writing application runtime files' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      generated = Pathname.new(directory).join('generated')
      copy_application(root)
      focus_scope = root.join('app/javascript/controllers/ruby_ui/focus_scope.js')
      focus_scope.write('export const focusScope = true\n')
      before = runtime_files(root)
      dependency_files_before = dependency_files(root)
      output = StringIO.new

      expect(RubyUiComparison.new(['--output', generated.to_s, '--all'], root:, stdout: output).run).to eq(0)

      expect_all_components_comparison(output, root, before, dependency_files_before, generated)
    end
  end

  it 'limits named-component local-only reporting to generated component paths' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      generated = Pathname.new(directory).join('generated')
      copy_application(root)
      local_only = root.join('app/components/ruby_ui/button/local_only.rb')
      local_only.dirname.mkpath
      File.write(local_only, "module RubyUI\n  class LocalOnly\n  end\nend\n")
      unrelated = root.join('app/components/ruby_ui/unrelated.rb')
      File.write(unrelated, "module RubyUI\n  class Unrelated\n  end\nend\n")

      output = StringIO.new
      expect(RubyUiComparison.new(['--output', generated.to_s, 'Button'], root:, stdout: output).run).to eq(0)

      expect(output.string).to include('Local-only files: app/components/ruby_ui/button/local_only.rb')
      expect(output.string).not_to include('app/components/ruby_ui/unrelated.rb')
    end
  end

  it 'reports the Bundler-activated locked generator version and source' do
    Dir.mktmpdir do |directory|
      generated = Pathname.new(directory).join('generated')

      output, error, status = compare('--output', generated.to_s, 'Button')

      expect(status).to be_success, error
      expect(output).to include('RubyUI version: 1.6.0')
      expect(output).to match(%r{Generator source: .*/ruby_ui-1\.6\.0})
    end
  end

  it 'reports dependency requests intercepted by the disposable command shims' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      output = Pathname.new(directory).join('generated')
      copy_application(root)
      comparison = RubyUiComparison.new(['--output', output.to_s, 'Button'], root:, stdout: StringIO.new)
      comparison.send(:copy_application, output)
      comparison.send(:install_recording_shims, output)

      _stdout, _error, status = Open3.capture3(output.join('bin/bundle').to_s, 'show', 'ruby_ui')
      expect(status).to be_success
      record_dependency_requests(output)

      expect_recorded_dependency_changes(comparison, output, root)
    end
  end

  it 'reports a real Codeblock dependency request without changing root manifests' do
    Dir.mktmpdir do |directory|
      generated = Pathname.new(directory).join('generated')
      before = dependency_files

      output, error, status = compare('--output', generated.to_s, 'Codeblock')

      expect(status).to be_success, error
      expect(output).to include('Dependency or controller-registration changes: gem rouge')
      expect(dependency_files).to eq(before)
    end
  end

  def compare(*)
    Open3.capture3('ruby', command_path.to_s, *, chdir: Rails.root)
  end

  def command_path
    Rails.root.join('scripts/compare_ruby_ui.rb')
  end

  def tracked_application_diff
    output, status = Open3.capture2('git', 'diff', '--name-only', '--', 'app', chdir: Rails.root)
    raise 'Could not inspect tracked application changes' unless status.success?

    output
  end

  def runtime_files(root = Rails.root)
    paths = %w[app/components/ruby_ui app/javascript/controllers/ruby_ui]
    paths.index_with do |path|
      root.join(path).glob('**/*').select(&:file?).to_h do |file|
        [file.relative_path_from(root).to_s, file.binread]
      end
    end
  end

  def dependency_files(root = Rails.root)
    %w[Gemfile Gemfile.lock package.json yarn.lock config/importmap.rb].to_h do |path|
      file = root.join(path)
      [path, file.exist? ? file.binread : nil]
    end
  end

  def checkout_dirty?
    output, status = Open3.capture2('git', 'status', '--porcelain', chdir: Rails.root)
    raise 'Could not inspect checkout status' unless status.success?

    !output.empty?
  end

  def copy_application(destination)
    destination.mkpath
    %w[app bin config lib Gemfile Gemfile.lock Rakefile].each do |entry|
      FileUtils.cp_r(Rails.root.join(entry), destination)
    end
  end

  def run_git(root, *)
    _output, error, status = Open3.capture3('git', *, chdir: root)
    raise error unless status.success?
  end

  def expect_isolated_comparison_output(output)
    expect(output).to include('RubyUI version: 1.6.0')
    expect(output).to include('Generator: bin/rails generate ruby_ui:component')
    expect(output).to include('Requested components: Button')
    expect(output).to include('Generated files:', 'Changed files:')
    expect(output).to include('Dependency or controller-registration changes:')
  end

  def expect_isolated_comparison_state(error, status, generated)
    expect(checkout_dirty?).to be(true)
    expect(status).to be_success, error
    expect(generated).to exist
  end

  def copy_generated_button(generated, root)
    FileUtils.cp(
      generated.join('app/components/ruby_ui/button/button.rb'),
      root.join('app/components/ruby_ui/button/button.rb')
    )
  end

  def copy_generated_dialog(generated, root)
    FileUtils.cp_r(generated.join('app/components/ruby_ui/dialog'), root.join('app/components/ruby_ui'))
    FileUtils.cp(
      generated.join('app/javascript/controllers/ruby_ui/dialog_controller.js'),
      root.join('app/javascript/controllers/ruby_ui/dialog_controller.js')
    )
  end

  def expect_all_components_comparison(output, root, before, dependency_files_before, generated)
    expect_all_components_output(output, generated, dependency_files_before)
    expect_all_components_state(root, before, dependency_files_before)
  end

  def expect_all_components_output(output, generated, dependency_files_before)
    expect(output.string).to include('Requested components: all')
    expect(output.string).to include('Dependency or controller-registration changes: config/importmap.rb')
    expect(output.string).to match(%r{Local-only files: .*app/javascript/controllers/ruby_ui/focus_scope\.js})
    expect_generated_importmap_changed(output, generated, dependency_files_before)
  end

  def expect_generated_importmap_changed(output, generated, dependency_files_before)
    expect(generated_files(output)).not_to include('app/javascript/controllers/ruby_ui/focus_scope.js')
    expect(generated.join('config/importmap.rb').binread).not_to eq(
      dependency_files_before.fetch('config/importmap.rb')
    )
  end

  def expect_all_components_state(root, before, dependency_files_before)
    expect(runtime_files(root)).to eq(before)
    expect(dependency_files(root)).to eq(dependency_files_before)
  end

  def generated_files(output)
    output.string.lines.find { it.start_with?('Generated files:') }
  end

  def record_dependency_requests(output)
    Open3.capture3(output.join('bin/bundle').to_s, 'add', 'example-gem')
    Open3.capture3(output.join('bin/importmap').to_s, 'pin', 'example-package')
    File.open(output.join('config/importmap.rb'), 'a') { it.puts('pin "already-pinned"') }
    Open3.capture3(output.join('bin/importmap').to_s, 'pin', 'already-pinned')
  end

  def expect_recorded_dependency_changes(comparison, output, root)
    expect_recorded_dependencies(comparison, output)
    expect_root_manifests_unchanged(output, root)
  end

  def expect_recorded_dependencies(comparison, output)
    expect(comparison.send(:recorded_dependency_changes, output)).to eq(
      ['gem example-gem', 'javascript example-package']
    )
  end

  def expect_root_manifests_unchanged(output, root)
    expect(output.join('Gemfile').binread).to eq(root.join('Gemfile').binread)
    expect(output.join('Gemfile.lock').binread).to eq(root.join('Gemfile.lock').binread)
  end
end
