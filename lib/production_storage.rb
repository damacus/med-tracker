# frozen_string_literal: true

module ProductionStorage
  class ConfigurationError < StandardError; end

  Configuration = Data.define(:service, :root)

  SERVICE = 'persistent'
  DEFAULT_ROOT = '/app/storage'
  DEFAULT_MOUNTINFO_PATH = '/proc/self/mountinfo'

  def self.resolve(environment: ENV, mountinfo_path: Pathname(DEFAULT_MOUNTINFO_PATH))
    service = service_name(environment)
    root = storage_root(environment)
    validate_mount!(root, mountinfo_path) unless asset_compilation?(environment)

    Configuration.new(service: service.to_sym, root: root)
  rescue Errno::ENOENT, Errno::EACCES => e
    raise ConfigurationError, "Unable to validate ACTIVE_STORAGE_ROOT: #{e.message}"
  end

  def self.service_name(environment)
    service = environment.fetch('ACTIVE_STORAGE_SERVICE', SERVICE)
    unless service == SERVICE
      raise ConfigurationError, "ACTIVE_STORAGE_SERVICE must be #{SERVICE.inspect} in production"
    end

    service
  end
  private_class_method :service_name

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
