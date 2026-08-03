# frozen_string_literal: true

module ProductionStorage
  class ConfigurationError < StandardError; end

  Configuration = Data.define(:service, :root)

  SERVICES = %w[
    persistent
    persistent_with_s3_mirror
    s3_with_persistent_mirror
    s3
  ].freeze
  DISK_SERVICES = %w[persistent persistent_with_s3_mirror s3_with_persistent_mirror].freeze
  S3_SERVICES = %w[persistent_with_s3_mirror s3_with_persistent_mirror s3].freeze
  S3_SETTINGS = %w[
    ACTIVE_STORAGE_S3_ENDPOINT
    ACTIVE_STORAGE_S3_BUCKET
    ACTIVE_STORAGE_S3_REGION
    ACTIVE_STORAGE_S3_ACCESS_KEY_ID
    ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY
  ].freeze
  DEFAULT_SERVICE = 'persistent'
  DEFAULT_ROOT = '/app/storage'
  DEFAULT_MOUNTINFO_PATH = '/proc/self/mountinfo'

  def self.resolve(environment: ENV, mountinfo_path: Pathname(DEFAULT_MOUNTINFO_PATH))
    service = service_name(environment)
    root = storage_root(environment) if DISK_SERVICES.include?(service)
    validate_mount!(root, mountinfo_path) if root && !asset_compilation?(environment)
    validate_s3!(environment) if S3_SERVICES.include?(service)

    Configuration.new(service: service.to_sym, root: root)
  rescue Errno::ENOENT, Errno::EACCES => e
    raise ConfigurationError, "Unable to validate ACTIVE_STORAGE_ROOT: #{e.message}"
  end

  def self.service_name(environment)
    service = environment.fetch('ACTIVE_STORAGE_SERVICE', DEFAULT_SERVICE)
    return service if SERVICES.include?(service)

    raise ConfigurationError, "ACTIVE_STORAGE_SERVICE must be one of #{SERVICES.join(', ')} in production"
  end
  private_class_method :service_name

  def self.validate_s3!(environment)
    missing_setting = S3_SETTINGS.find { |name| environment[name].to_s.strip.empty? }
    raise ConfigurationError, "#{missing_setting} is required" if missing_setting
  end
  private_class_method :validate_s3!

  def self.storage_root(environment)
    root = Pathname(environment.fetch('ACTIVE_STORAGE_ROOT', DEFAULT_ROOT)).cleanpath
    raise ConfigurationError, 'ACTIVE_STORAGE_ROOT must be an absolute path' unless root.absolute?
    raise ConfigurationError, 'ACTIVE_STORAGE_ROOT must name an existing directory' unless root.directory?
    raise ConfigurationError, 'ACTIVE_STORAGE_ROOT must be writable' unless root.writable?

    root.realpath
  end
  private_class_method :storage_root

  def self.asset_compilation?(environment)
    environment['SECRET_KEY_BASE_DUMMY'] == '1'
  end
  private_class_method :asset_compilation?

  def self.validate_mount!(root, mountinfo_path)
    mounted = mountinfo_path.each_line.any? do |line|
      mount_path = line.split.fetch(4)
      Pathname(unescape_mount_path(mount_path)).cleanpath == root
    end
    return if mounted

    raise ConfigurationError, 'ACTIVE_STORAGE_ROOT must be a mounted persistent volume'
  end
  private_class_method :validate_mount!

  def self.unescape_mount_path(path)
    path.gsub(/\\([0-7]{3})/) { Regexp.last_match(1).to_i(8).chr }
  end
  private_class_method :unescape_mount_path
end
