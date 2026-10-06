# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('lib/production_storage')

RSpec.describe ProductionStorage do
  let(:s3_environment) do
    {
      'ACTIVE_STORAGE_S3_ENDPOINT' => 'https://objects.example.test',
      'ACTIVE_STORAGE_S3_BUCKET' => 'medtracker-production',
      'ACTIVE_STORAGE_S3_REGION' => 'local',
      'ACTIVE_STORAGE_S3_ACCESS_KEY_ID' => 'access-key-marker',
      'ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY' => 'secret-key-marker'
    }
  end

  def resolve(service: 'persistent', root: nil, mounted: true, dummy: false, environment: {})
    mountinfo = Tempfile.new('mountinfo')
    mountinfo.write("36 25 0:32 / #{root} rw,relatime - ext4 /dev/test rw\n") if mounted && root
    mountinfo.close

    resolved_environment = environment.merge('ACTIVE_STORAGE_SERVICE' => service)
    resolved_environment['ACTIVE_STORAGE_ROOT'] = root.to_s if root
    resolved_environment['SECRET_KEY_BASE_DUMMY'] = '1' if dummy

    described_class.resolve(environment: resolved_environment, mountinfo_path: Pathname(mountinfo.path))
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

  it 'selects the disk-primary mirror with both backend configurations' do
    Dir.mktmpdir do |directory|
      configuration = resolve(
        service: 'persistent_with_s3_mirror',
        root: Pathname(directory),
        environment: s3_environment
      )

      expect(configuration.service).to eq(:persistent_with_s3_mirror)
      expect(configuration.root).to eq(Pathname(directory).realpath)
    end
  end

  it 'selects the S3-primary mirror with both backend configurations' do
    Dir.mktmpdir do |directory|
      configuration = resolve(
        service: 's3_with_persistent_mirror',
        root: Pathname(directory),
        environment: s3_environment
      )

      expect(configuration.service).to eq(:s3_with_persistent_mirror)
      expect(configuration.root).to eq(Pathname(directory).realpath)
    end
  end

  it 'selects S3 without requiring a disk root or mount' do
    configuration = resolve(service: 's3', mounted: false, environment: s3_environment)

    expect(configuration.service).to eq(:s3)
    expect(configuration.root).to be_nil
  end

  it 'does not require S3 settings for persistent disk' do
    Dir.mktmpdir do |directory|
      expect { resolve(root: Pathname(directory)) }.not_to raise_error
    end
  end

  it 'requires every S3 setting only for S3-inclusive services' do
    %w[
      ACTIVE_STORAGE_S3_ENDPOINT
      ACTIVE_STORAGE_S3_BUCKET
      ACTIVE_STORAGE_S3_REGION
      ACTIVE_STORAGE_S3_ACCESS_KEY_ID
      ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY
    ].each do |setting|
      incomplete_environment = s3_environment.except(setting)

      expect { resolve(service: 's3', mounted: false, environment: incomplete_environment) }
        .to raise_error(described_class::ConfigurationError, /#{setting} is required/)
    end
  end

  it 'does not disclose configured credentials when S3 validation fails' do
    environment = s3_environment.except('ACTIVE_STORAGE_S3_BUCKET')

    expect { resolve(service: 's3', mounted: false, environment: environment) }
      .to raise_error(described_class::ConfigurationError, /ACTIVE_STORAGE_S3_BUCKET is required/) do |error|
        expect(error.message).not_to include('access-key-marker', 'secret-key-marker')
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
