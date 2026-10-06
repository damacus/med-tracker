# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::StorageCleaner do
  let(:disk_cleaner) { instance_double(DemoReset::DiskStorageCleaner, call: { files_removed: 2 }, empty?: true) }
  let(:s3_cleaner) { instance_double(DemoReset::S3StorageCleaner, call: { objects_removed: 3 }, empty?: true) }

  {
    'persistent' => %i[disk],
    's3' => %i[s3],
    'persistent_with_s3_mirror' => %i[disk s3],
    's3_with_persistent_mirror' => %i[disk s3]
  }.each do |service_name, backends|
    it "cleans the configured #{service_name} backend" do
      result = cleaner(service_name:).call

      expect(result).to eq(expected_result(backends))
      expect(disk_cleaner).to have_received(:call).exactly(backends.count(:disk)).times
      expect(s3_cleaner).to have_received(:call).exactly(backends.count(:s3)).times
    end

    it "checks whether the configured #{service_name} backend is empty" do
      expect(cleaner(service_name:)).to be_empty

      expect(disk_cleaner).to have_received(:empty?).exactly(backends.count(:disk)).times
      expect(s3_cleaner).to have_received(:empty?).exactly(backends.count(:s3)).times
    end
  end

  it 'refuses to clean when the configured service differs from the expected service' do
    expect { cleaner(service_name: 's3', expected_service_name: 'persistent').call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_service/)
    expect(disk_cleaner).not_to have_received(:call)
    expect(s3_cleaner).not_to have_received(:call)
  end

  it 'refuses an unsupported configured service' do
    expect { cleaner(service_name: 'test', expected_service_name: 'test').call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_service/)
  end

  def cleaner(service_name:, expected_service_name: service_name)
    described_class.new(service_name:, expected_service_name:, disk_cleaner:, s3_cleaner:)
  end

  def expected_result(backends)
    {}.tap do |result|
      result[:files_removed] = 2 if backends.include?(:disk)
      result[:objects_removed] = 3 if backends.include?(:s3)
    end
  end
end
