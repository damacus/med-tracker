# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::DiskStorageCleaner do
  let(:storage_root) { Pathname(Dir.mktmpdir('demo-reset-storage')) }

  after { FileUtils.rm_rf(storage_root) }

  it 'removes every entry below the verified mount and leaves the root in place' do
    storage_root.join('nested').mkpath
    storage_root.join('nested/upload.bin').write('synthetic upload')
    storage_root.join('orphan.bin').write('synthetic orphan')

    result = cleaner.call

    expect(result).to eq(files_removed: 2)
    expect(storage_root).to be_directory
    expect(storage_root.children).to be_empty
  end

  it 'can be retried safely after post-commit cleanup fails' do
    storage_root.join('first.bin').write('synthetic first')
    storage_root.join('second.bin').write('synthetic second')
    attempts = 0
    remover = lambda do |path|
      attempts += 1
      raise Errno::EIO, 'cleanup failed' if attempts == 1

      FileUtils.rm_rf(path)
    end

    expect { cleaner(remover:).call }.to raise_error(DemoReset::StorageCleanupError, 'storage cleanup failed')
    expect(cleaner.call).to eq(files_removed: 2)
    expect(storage_root.children).to be_empty
  end

  it 'refuses a configured root that resolves outside the expected mount' do
    outside_root = Pathname(Dir.mktmpdir('outside-demo-storage'))
    link = storage_root.dirname.join("#{storage_root.basename}-link")
    FileUtils.ln_s(outside_root, link)

    expect do
      described_class.new(root: link, expected_root: storage_root, mount_checker: ->(_) { true }).call
    end.to raise_error(DemoReset::UnsafeTargetError, /storage_root/)
  ensure
    FileUtils.rm_f(link) if link
    FileUtils.rm_rf(outside_root) if outside_root
  end

  it 'refuses a root that is not the expected persistent mount' do
    storage_root.join('upload.bin').write('synthetic upload')

    expect { cleaner(mount_checker: ->(_) { false }).call }
      .to raise_error(DemoReset::UnsafeTargetError, /storage_mount/)
    expect(storage_root.join('upload.bin')).to exist
  end

  it 'reports whether the verified root is empty' do
    storage_root.join('upload.bin').write('synthetic upload')
    expect(cleaner).not_to be_empty

    storage_root.children.each { |entry| FileUtils.rm_rf(entry) }

    expect(cleaner).to be_empty
  end

  def cleaner(remover: FileUtils.method(:rm_rf), mount_checker: ->(_) { true })
    described_class.new(root: storage_root, expected_root: storage_root, remover:, mount_checker:)
  end
end
