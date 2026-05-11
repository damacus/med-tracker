# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SystemMetadata do
  describe '.current' do
    after do
      FileUtils.remove_entry(directory) if directory.exist?
    end

    let(:directory) { Pathname.new(Dir.mktmpdir) }
    let(:root) { directory.join('worktree') }
    let(:git_dir) { directory.join('repo/.git/worktrees/worktree') }

    it 'resolves the linked worktree path from gitdir metadata and the short commit' do
      create_linked_gitdir

      metadata = described_class.current(
        root:,
        command_runner: command_runner(
          ['git', 'rev-parse', '--path-format=absolute', '--git-dir'] => git_dir.to_s,
          ['git', 'rev-parse', '--short', 'HEAD'] => 'bb956afc'
        )
      )

      expect(metadata.worktree).to eq(root.to_s)
      expect(metadata.commit).to eq('bb956afc')
    end

    it 'falls back to the git top-level worktree when linked metadata is unavailable' do
      FileUtils.mkdir_p(root)

      metadata = described_class.current(
        root:,
        command_runner: command_runner(
          ['git', 'rev-parse', '--path-format=absolute', '--git-dir'] => SystemMetadata::CommandError.new,
          ['git', 'rev-parse', '--show-toplevel'] => root.to_s,
          ['git', 'rev-parse', '--short', 'HEAD'] => 'bb956afc'
        )
      )

      expect(metadata.worktree).to eq(root.to_s)
      expect(metadata.commit).to eq('bb956afc')
    end

    it 'falls back to Rails.root and an unknown commit when git metadata is unavailable' do
      root = Pathname.new('/app')

      metadata = described_class.current(
        root:,
        command_runner: ->(*_command) { raise SystemMetadata::CommandError }
      )

      expect(metadata.worktree).to eq('/app')
      expect(metadata.commit).to eq('unknown')
    end

    def create_linked_gitdir
      FileUtils.mkdir_p(root)
      FileUtils.mkdir_p(git_dir)
      File.write(root.join('.git'), "gitdir: #{git_dir}\n")
      File.write(git_dir.join('gitdir'), "#{root.join('.git')}\n")
    end

    def command_runner(responses)
      lambda do |*command|
        response = responses.fetch(command) { raise "Unexpected command: #{command.join(' ')}" }

        raise response if response.is_a?(Exception)

        response
      end
    end
  end
end
