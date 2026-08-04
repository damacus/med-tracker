# frozen_string_literal: true

require 'fileutils'

module DemoReset
  class DiskStorageCleaner
    MOUNTINFO_PATH = Pathname(ProductionStorage::DEFAULT_MOUNTINFO_PATH)

    def initialize(root: ENV.fetch('ACTIVE_STORAGE_ROOT', ProductionStorage::DEFAULT_ROOT),
                   expected_root: ENV.fetch('DEMO_RESET_EXPECTED_STORAGE_ROOT', nil),
                   remover: FileUtils.method(:rm_rf), mount_checker: method(:mounted?))
      @root = Pathname(root.to_s)
      @expected_root = Pathname(expected_root.to_s)
      @remover = remover
      @mount_checker = mount_checker
    end

    def call
      verified_root = verify_target!
      files_removed = file_count(verified_root)
      verified_root.children.each { |entry| remover.call(entry) }
      raise StorageCleanupError, 'storage cleanup failed' unless verified_root.children.empty?

      { files_removed: }
    rescue UnsafeTargetError, StorageCleanupError
      raise
    rescue SystemCallError
      raise StorageCleanupError, 'storage cleanup failed'
    end

    def empty?
      verify_target!.children.empty?
    rescue UnsafeTargetError, StorageCleanupError
      raise
    rescue SystemCallError
      raise StorageCleanupError, 'storage cleanup failed'
    end

    private

    attr_reader :root, :expected_root, :remover, :mount_checker

    def verify_target!
      raise UnsafeTargetError, 'demo reset refused: storage_root' unless root.directory? && expected_root.directory?

      resolved_root = root.realpath
      expected = expected_root.realpath
      raise UnsafeTargetError, 'demo reset refused: storage_root' unless resolved_root == expected
      raise UnsafeTargetError, 'demo reset refused: storage_mount' unless mount_checker.call(resolved_root)

      resolved_root
    end

    def file_count(directory)
      directory.glob('**/*', File::FNM_DOTMATCH).count { |path| path.file? || path.symlink? }
    end

    def mounted?(directory)
      MOUNTINFO_PATH.each_line.any? do |line|
        Pathname(unescape_mount_path(line.split.fetch(4))).cleanpath == directory
      end
    rescue Errno::ENOENT, Errno::EACCES, IndexError
      false
    end

    def unescape_mount_path(path)
      path.gsub(/\\([0-7]{3})/) { Regexp.last_match(1).to_i(8).chr }
    end
  end
end
