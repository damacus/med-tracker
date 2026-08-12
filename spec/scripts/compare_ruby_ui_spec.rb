# frozen_string_literal: true

require 'rails_helper'
require 'open3'
require 'stringio'
require 'tmpdir'
require Rails.root.join('scripts/compare_ruby_ui')

# rubocop:disable RSpec/DescribeClass
RSpec.describe 'RubyUI comparison command' do
  # rubocop:enable RSpec/DescribeClass
  it 'requires at least one explicitly named component family' do
    _output, error, status = compare

    expect(status).not_to be_success
    expect(error).to include('Pass one or more RubyUI component names.')
  end

  it 'does not support whole-library generation' do
    _output, error, status = compare('--all')

    expect(status).not_to be_success
    expect(error).to include('invalid option: --all')
  end

  it 'reports a selected component difference from the real generator' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('generated')
      before = protected_files

      stdout, error, status = compare('--output', output.to_s, 'Button')

      expect(status).to be_success, error
      expect(stdout).to include('Changed files: app/components/ruby_ui/button/button.rb')
      expect(stdout).to include('Requested components: Button')
      expect(protected_files).to eq(before)
    end
  end

  it 'reports unchanged and local-only files only for the requested family' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      first_output = Pathname.new(directory).join('first')
      second_output = Pathname.new(directory).join('second')
      copy_application(root)
      prepare_matching_button(root, first_output)
      add_local_only_fixtures(root)

      status, stdout, error = run_comparison(root, ['--output', second_output.to_s, 'Button'])

      expect(status).to eq(0), error
      expect_named_button_scope(stdout)
    end
  end

  it 'rejects an invalid component before creating output' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('generated')

      _stdout, error, status = compare('--output', output.to_s, 'NotARealRubyUiComponent')

      expect(status).not_to be_success
      expect(error).to include('Unknown RubyUI component: NotARealRubyUiComponent')
      expect(output).not_to exist
    end
  end

  it 'rejects component names containing path segments' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('generated')

      _stdout, error, status = compare('--output', output.to_s, '../generators/ruby_ui')

      expect(status).not_to be_success
      expect(error).to include('Unknown RubyUI component: ../generators/ruby_ui')
      expect(output).not_to exist
    end
  end

  it 'creates the disposable generator workspace outside the checkout even when TMPDIR points inside it' do
    temporary_parent = Rails.root.join('tmp/ruby-ui-comparison-red')
    temporary_parent.mkpath
    comparison = RubyUiComparison.new(['Button'], root: Rails.root)
    workspace = comparison.send(:create_workspace)

    expect(workspace.realpath.to_s).not_to start_with(Rails.root.realpath.to_s)
    expect(temporary_parent.children).to be_empty
  ensure
    FileUtils.remove_entry(workspace) if workspace&.exist?
    temporary_parent.rmdir if temporary_parent&.directory?
  end

  it 'rejects both output option spellings inside the checkout' do
    arguments = [
      ['--output', Rails.root.join('generated').to_s, 'Button'],
      ["--output=#{Rails.root.join('generated')}", 'Button']
    ]

    arguments.each do |command_arguments|
      _stdout, error, status = compare(*command_arguments)

      expect(status).not_to be_success
      expect(error).to include('must be outside the application checkout')
    end
  end

  it 'rejects an existing output path' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('existing')
      output.mkpath

      _stdout, error, status = compare('--output', output.to_s, 'Button')

      expect(status).not_to be_success
      expect(error).to include('must not already exist')
    end
  end

  it 'rejects output paths that are or traverse symbolic links' do
    Dir.mktmpdir do |directory|
      destination = Pathname.new(directory).join('destination')
      destination.mkpath
      direct_link = Pathname.new(directory).join('direct-link')
      parent_link = Pathname.new(directory).join('parent-link')
      File.symlink(destination, direct_link)
      File.symlink(destination, parent_link)

      direct = compare('--output', direct_link.to_s, 'Button')
      traversed = compare("--output=#{parent_link.join('generated')}", 'Button')

      expect(direct[1]).to include('must not already exist')
      expect(traversed[1]).to include('must not traverse symbolic links')
      expect(direct[2]).not_to be_success
      expect(traversed[2]).not_to be_success
    end
  end

  it 'handles spaces and shell metacharacters as literal output-path characters' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('RubyUI output ; $() [safe]')

      stdout, error, status = compare("--output=#{output}", 'Button')

      expect(status).to be_success, error
      expect(stdout).to include('Requested components: Button')
      expect(output.join('app/components/ruby_ui/button/button.rb')).to exist
    end
  end

  it 'reports dependency metadata without installing requested dependencies' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('generated')
      before = protected_files

      stdout, error, status = compare('--output', output.to_s, 'Codeblock')

      expect(status).to be_success, error
      expect(stdout).to include(
        'Declared dependencies (Codeblock): components=Button, Clipboard; gems=rouge; javascript=none'
      )
      expect(output.join('app/components/ruby_ui/button')).not_to exist
      expect(output.join('app/components/ruby_ui/clipboard')).not_to exist
      expect(protected_files).to eq(before)
    end
  end

  it 'uses the Bundler-activated RubyUI 1.6.0 generator' do
    Dir.mktmpdir do |directory|
      output = Pathname.new(directory).join('generated')

      stdout, error, status = compare('--output', output.to_s, 'Dialog')

      expect(status).to be_success, error
      expect(stdout).to include('RubyUI version: 1.6.0')
      expect(stdout).to match(%r{Generator source: .*/ruby_ui-1\.6\.0})
      expect(output.join('app/javascript/controllers/ruby_ui/dialog_controller.js')).to exist
    end
  end

  def compare(*)
    Open3.capture3('ruby', Rails.root.join('scripts/compare_ruby_ui.rb').to_s, *, chdir: Rails.root)
  end

  def run_comparison(root, arguments)
    stdout = StringIO.new
    stderr = StringIO.new
    status = RubyUiComparison.new(arguments, root:, stdout:, stderr:).run
    [status, stdout.string, stderr.string]
  end

  def copy_application(destination)
    destination.mkpath
    %w[app bin config lib Gemfile Gemfile.lock Rakefile].each do |entry|
      FileUtils.cp_r(Rails.root.join(entry), destination)
    end
  end

  def copy_generated_family(generated, root, family)
    FileUtils.rm_rf(root.join('app/components/ruby_ui', family))
    FileUtils.cp_r(generated.join('app/components/ruby_ui', family), root.join('app/components/ruby_ui'))
  end

  def prepare_matching_button(root, generated)
    expect(run_comparison(root, ['--output', generated.to_s, 'Button']).first).to eq(0)
    copy_generated_family(generated, root, 'button')
  end

  def add_local_only_fixtures(root)
    root.join('app/components/ruby_ui/button/local_only.rb').write("module RubyUI\n  class LocalOnly\n  end\nend\n")
    root.join('app/components/ruby_ui/dialog/unrelated.rb').write("module RubyUI\n  class Unrelated\n  end\nend\n")
    button_controller = root.join('app/javascript/controllers/ruby_ui/button_custom_controller.js')
    dialog_controller = root.join('app/javascript/controllers/ruby_ui/dialog_custom_controller.js')
    button_controller.write('export default class ButtonCustomController {}')
    dialog_controller.write('export default class DialogCustomController {}')
  end

  def expect_named_button_scope(stdout)
    expect(stdout).to include('Unchanged files: app/components/ruby_ui/button/button.rb')
    expect(stdout).to include('Local-only files: app/components/ruby_ui/button/local_only.rb')
    expect(stdout).to include('app/javascript/controllers/ruby_ui/button_custom_controller.js')
    expect(stdout).not_to include('app/components/ruby_ui/dialog/unrelated.rb')
    expect(stdout).not_to include('app/javascript/controllers/ruby_ui/dialog_custom_controller.js')
  end

  def protected_files
    paths = %w[
      app/components/ruby_ui app/javascript/controllers/ruby_ui Gemfile Gemfile.lock package.json yarn.lock
      config/importmap.rb
    ]
    paths.to_h do |path|
      target = Rails.root.join(path)
      files = if target.directory?
                target.glob('**/*').select(&:file?)
              elsif target.file?
                [target]
              else
                []
              end
      [path, files.to_h { |file| [file.relative_path_from(Rails.root).to_s, file.binread] }]
    end
  end
end
