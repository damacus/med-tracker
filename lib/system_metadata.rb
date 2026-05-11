# frozen_string_literal: true

require 'open3'

SystemMetadata = Data.define(:worktree, :commit) do
  def self.current(root: Rails.root, command_runner: nil)
    new(
      worktree: worktree_path(root:, command_runner:),
      commit: commit_sha(root:, command_runner:)
    )
  end

  def self.worktree_path(root:, command_runner:)
    linked_worktree_path(root:, command_runner:) || git_toplevel(root:, command_runner:) || root.to_s
  end

  def self.linked_worktree_path(root:, command_runner:)
    git_dir = run_git(root:, command_runner:, args: ['rev-parse', '--path-format=absolute', '--git-dir'])
    gitdir_file = Pathname.new(git_dir).join('gitdir')

    return unless gitdir_file.file?

    worktree_git_file = Pathname.new(gitdir_file.read.strip)
    return if worktree_git_file.to_s.blank?

    worktree_git_file.dirname.to_s
  rescue StandardError
    nil
  end

  def self.git_toplevel(root:, command_runner:)
    run_git(root:, command_runner:, args: ['rev-parse', '--show-toplevel'])
  rescue StandardError
    nil
  end

  def self.commit_sha(root:, command_runner:)
    run_git(root:, command_runner:, args: ['rev-parse', '--short', 'HEAD'])
  rescue StandardError
    'unknown'
  end

  def self.run_git(root:, command_runner:, args:)
    output = if command_runner
               command_runner.call('git', *args)
             else
               stdout, _stderr, status = Open3.capture3('git', *args, chdir: root.to_s)
               raise CommandError unless status.success?

               stdout
             end

    output.to_s.strip.presence || raise(CommandError)
  end

  private_class_method :worktree_path,
                       :linked_worktree_path,
                       :git_toplevel,
                       :commit_sha,
                       :run_git
end

class SystemMetadata
  class CommandError < StandardError; end
end
