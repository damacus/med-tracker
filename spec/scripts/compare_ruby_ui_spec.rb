require 'rails_helper'
require 'open3'
require 'tmpdir'
require 'stringio'
require Rails.root.join('scripts/compare_ruby_ui')

RSpec.describe 'RubyUI comparison command' do
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

  it 'permits explicit isolated output on a dirty checkout' do
    Dir.mktmpdir do |directory|
      output, error, status = compare('--output', directory, 'Button')

      expect(checkout_dirty?).to be(true)
      expect(status).to be_success, error
      expect(output).to include('RubyUI version: 1.6.0')
      expect(output).to include('Generator: bin/rails generate ruby_ui:component')
      expect(output).to include('Requested components: Button')
      expect(output).to include('Generated files:')
      expect(output).to include('Changed files:')
      expect(output).to include('Dependency or controller-registration changes:')
    end
  end

  it 'reports an unchanged component when its generated output matches local source' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      copy_application(root)
      generated = Pathname.new(directory).join('generated')
      first_output = StringIO.new

      expect(RubyUiComparison.new(['--output', generated.to_s, 'Button'], root:, stdout: first_output).run).to eq(0)
      FileUtils.cp(generated.join('app/components/ruby_ui/button/button.rb'), root.join('app/components/ruby_ui/button/button.rb'))

      second_output = StringIO.new
      expect(RubyUiComparison.new(['--output', generated.to_s, 'Button'], root:, stdout: second_output).run).to eq(0)
      expect(second_output.string).to include('Changed files: none')
    end
  end

  it 'reports a generated Stimulus controller difference' do
    Dir.mktmpdir do |directory|
      root = Pathname.new(directory).join('source')
      generated = Pathname.new(directory).join('generated')
      copy_application(root)

      expect(RubyUiComparison.new(['--output', generated.to_s, 'Dialog'], root:, stdout: StringIO.new).run).to eq(0)
      FileUtils.cp_r(generated.join('app/components/ruby_ui/dialog'), root.join('app/components/ruby_ui'))
      FileUtils.cp(
        generated.join('app/javascript/controllers/ruby_ui/dialog_controller.js'),
        root.join('app/javascript/controllers/ruby_ui/dialog_controller.js')
      )
      File.write(root.join('app/javascript/controllers/ruby_ui/dialog_controller.js'), 'local change')

      output = StringIO.new
      expect(RubyUiComparison.new(['--output', generated.to_s, 'Dialog'], root:, stdout: output).run).to eq(0)
      expect(output.string).to include('Changed files: app/javascript/controllers/ruby_ui/dialog_controller.js')
    end
  end

  it 'reports a missing component without changing tracked application files' do
    before = tracked_application_diff

    Dir.mktmpdir do |directory|
      _output, error, status = compare('--output', directory, 'NotARealRubyUiComponent')

      expect(status).not_to be_success
      expect(error).to include('RubyUI generator failed')
    end
    expect(tracked_application_diff).to eq(before)
  end

  it 'allows all-components comparison only through --all' do
    command = File.read(command_path)

    expect(command).to include("'ruby_ui:component:all'")
    expect(command).to include("options[:all]")
  end

  def compare(*arguments)
    Open3.capture3('ruby', command_path.to_s, *arguments, chdir: Rails.root)
  end

  def command_path
    Rails.root.join('scripts/compare_ruby_ui.rb')
  end

  def tracked_application_diff
    output, status = Open3.capture2('git', 'diff', '--name-only', '--', 'app', chdir: Rails.root)
    raise 'Could not inspect tracked application changes' unless status.success?

    output
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

  def run_git(root, *arguments)
    _output, error, status = Open3.capture3('git', *arguments, chdir: root)
    raise error unless status.success?
  end
end
