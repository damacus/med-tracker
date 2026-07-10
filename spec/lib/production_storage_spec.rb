# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('lib/production_storage')

RSpec.describe ProductionStorage do
  def resolve(root:, service: 'persistent', mounted: true, dummy: false)
    mountinfo = Tempfile.new('mountinfo')
    mountinfo.write("36 25 0:32 / #{root} rw,relatime - ext4 /dev/test rw\n") if mounted
    mountinfo.close

    environment = {
      'ACTIVE_STORAGE_SERVICE' => service,
      'ACTIVE_STORAGE_ROOT' => root.to_s
    }
    environment['SECRET_KEY_BASE_DUMMY'] = '1' if dummy

    described_class.resolve(environment: environment, mountinfo_path: Pathname(mountinfo.path))
  ensure
    mountinfo&.unlink
  end

  it 'selects the mounted persistent disk service' do
    Dir.mktmpdir do |directory|
      configuration = resolve(root: Pathname(directory))

      expect(configuration.service).to eq(:persistent)
      expect(configuration.root).to eq(Pathname(directory).realpath)
    end
  end

  it 'rejects unsupported production services' do
    Dir.mktmpdir do |directory|
      expect { resolve(root: Pathname(directory), service: 'local') }
        .to raise_error(described_class::ConfigurationError, /ACTIVE_STORAGE_SERVICE/)
    end
  end

  it 'rejects a relative storage root' do
    expect { resolve(root: Pathname('storage')) }
      .to raise_error(described_class::ConfigurationError, /absolute/)
  end

  it 'rejects a missing storage root' do
    root = Pathname(Dir.tmpdir).join("missing-storage-#{SecureRandom.hex(6)}")

    expect { resolve(root: root) }
      .to raise_error(described_class::ConfigurationError, /directory/)
  end

  it 'rejects a directory that is not a mounted volume' do
    Dir.mktmpdir do |directory|
      expect { resolve(root: Pathname(directory), mounted: false) }
        .to raise_error(described_class::ConfigurationError, /mounted persistent volume/)
    end
  end

  it 'allows production asset compilation without a runtime volume' do
    Dir.mktmpdir do |directory|
      configuration = resolve(root: Pathname(directory), mounted: false, dummy: true)

      expect(configuration.service).to eq(:persistent)
    end
  end
end
