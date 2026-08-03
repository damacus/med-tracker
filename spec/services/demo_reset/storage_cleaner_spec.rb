# frozen_string_literal: true

require 'rails_helper'
require 'aws-sdk-s3'

RSpec.describe DemoReset::StorageCleaner do
  let(:client) { instance_double(Aws::S3::Client) }
  let(:objects) { %w[active-storage-key orphan-key] }

  before do
    allow(client).to receive(:list_objects_v2) do
      Aws::S3::Types::ListObjectsV2Output.new(
        contents: objects.map { |key| Aws::S3::Types::Object.new(key:) },
        is_truncated: false
      )
    end
    allow(client).to receive(:delete_objects) do |delete:, **|
      objects.delete_if { |key| delete.fetch(:objects).pluck(:key).include?(key) }
      Aws::S3::Types::DeleteObjectsOutput.new(errors: [])
    end
  end

  it 'removes every object from the verified canary bucket' do
    result = cleaner.call

    expect(result).to eq(objects_removed: 2)
    expect(objects).to be_empty
    expect(client).to have_received(:delete_objects).with(hash_including(bucket: 'med-tracker-canary'))
  end

  it 'can be retried safely after post-commit cleanup fails' do
    allow(client).to receive(:delete_objects).once.and_raise(
      Aws::S3::Errors::InternalError.new(nil, 'unsafe provider detail')
    )

    expect { cleaner.call }.to raise_error(DemoReset::StorageCleanupError, 'storage cleanup failed')

    allow(client).to receive(:delete_objects) do |delete:, **|
      objects.delete_if { |key| delete.fetch(:objects).pluck(:key).include?(key) }
      Aws::S3::Types::DeleteObjectsOutput.new(errors: [])
    end
    expect(cleaner.call).to eq(objects_removed: 2)
    expect(objects).to be_empty
  end

  it 'follows continuation tokens so every object is removed' do
    first_page = Aws::S3::Types::ListObjectsV2Output.new(
      contents: [Aws::S3::Types::Object.new(key: 'first-key')],
      is_truncated: true,
      next_continuation_token: 'next-page'
    )
    second_page = Aws::S3::Types::ListObjectsV2Output.new(
      contents: [Aws::S3::Types::Object.new(key: 'second-key')],
      is_truncated: false
    )
    empty_page = Aws::S3::Types::ListObjectsV2Output.new(contents: [], is_truncated: false)
    allow(client).to receive(:list_objects_v2).and_return(first_page, second_page, empty_page)
    allow(client).to receive(:delete_objects).and_return(Aws::S3::Types::DeleteObjectsOutput.new(errors: []))

    expect(cleaner.call).to eq(objects_removed: 2)
    expect(client).to have_received(:list_objects_v2)
      .with(bucket: 'med-tracker-canary', continuation_token: 'next-page')
  end

  it 'fails safely when the provider reports an object deletion error' do
    provider_error = Aws::S3::Types::Error.new(key: 'secret-object-key', message: 'credential detail')
    allow(client).to receive(:delete_objects)
      .and_return(Aws::S3::Types::DeleteObjectsOutput.new(errors: [provider_error]))

    expect { cleaner.call }.to raise_error(DemoReset::StorageCleanupError, 'storage cleanup failed')
  end

  it 'refuses any bucket other than the isolated canary bucket' do
    expect { cleaner(bucket: 'med-tracker-production').call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_bucket/)
    expect(client).not_to have_received(:list_objects_v2)
  end

  it 'refuses any endpoint other than the in-cluster object store' do
    expect { cleaner(endpoint: 'https://production-objects.example.test').call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_endpoint/)
    expect(client).not_to have_received(:list_objects_v2)
  end

  it 'reports whether the verified bucket is empty' do
    expect(cleaner).not_to be_empty

    objects.clear

    expect(cleaner).to be_empty
  end

  def cleaner(bucket: 'med-tracker-canary', endpoint: 'http://rustfs.storage.svc.cluster.local:9000')
    described_class.new(client:, service: 's3', bucket:, endpoint:)
  end
end
