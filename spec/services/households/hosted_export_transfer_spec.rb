# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'
require 'tmpdir'

RSpec.describe Households::HostedExportTransfer do
  let(:household) { create(:household) }
  let(:actor) do
    Account.create!(email: 'hosted-export-transfer@example.test', status: :verified)
  end
  let(:membership) do
    household.household_memberships.create!(
      account: actor,
      role: :owner,
      status: :active,
      joined_at: Time.current
    )
  end
  let(:export) do
    Households::HostedExport.generate!(household: household, membership: membership, actor_account: actor)
  end

  it 'rejects a destination outside the configured output root before downloading' do
    Dir.mktmpdir('hosted-export-transfer') do |directory|
      output_root = File.join(directory, 'allowed')
      FileUtils.mkdir(output_root)
      destination = File.join(directory, 'outside.zip')
      previous_root = ENV.fetch('HOUSEHOLD_EXPORT_OUTPUT_ROOT', nil)
      ENV['HOUSEHOLD_EXPORT_OUTPUT_ROOT'] = output_root

      expect do
        described_class.call(export: export, actor_account: actor, destination: destination)
      end.to raise_error(ArgumentError, 'Destination must be inside the configured export output root')
      expect(File).not_to exist(destination)
      expect(export.reload).to be_ready
      expect(SecurityAuditEvent.where(household: household, event_type: 'household.export.downloaded')).to be_empty
    ensure
      restore_output_root(previous_root)
    end
  end

  it 'never overwrites an existing operator destination' do
    Dir.mktmpdir('hosted-export-transfer') do |output_root|
      destination = File.join(output_root, 'existing.zip')
      File.binwrite(destination, 'operator-owned-existing-bytes')
      previous_root = ENV.fetch('HOUSEHOLD_EXPORT_OUTPUT_ROOT', nil)
      ENV['HOUSEHOLD_EXPORT_OUTPUT_ROOT'] = output_root

      expect do
        described_class.call(export: export, actor_account: actor, destination: destination)
      end.to raise_error(Errno::EEXIST)
      expect(File.binread(destination)).to eq('operator-owned-existing-bytes')
      expect(export.reload).to be_ready
    ensure
      restore_output_root(previous_root)
    end
  end

  it 'removes the destination and leaves no download transition when checksum verification fails' do
    with_output_destination('checksum-failure.zip') do |destination|
      export.update!(artifact_checksum_sha256: 'invalid-checksum')

      expect do
        described_class.call(export: export, actor_account: actor, destination: destination)
      end.to raise_error(ActiveStorage::IntegrityError)

      expect_failed_transfer(destination)
    end
  end

  it 'removes the destination and leaves no download transition when durable write fails' do
    with_output_destination('fsync-failure.zip') do |destination|
      allow(File).to receive(:open).and_wrap_original do |original, *arguments, &block|
        original.call(*arguments) do |file|
          allow(file).to receive(:fsync).and_raise(IOError, 'fsync failed')
          block.call(file)
        end
      end

      expect do
        described_class.call(export: export, actor_account: actor, destination: destination)
      end.to raise_error(IOError, 'fsync failed')

      expect_failed_transfer(destination)
    end
  end

  it 'removes the destination and leaves no download transition after a partial write' do
    with_output_destination('partial-write.zip') do |destination|
      allow(File).to receive(:open).and_wrap_original do |original, *arguments, &block|
        original.call(*arguments) do |file|
          allow(file).to receive(:write).and_return(1)
          block.call(file)
        end
      end

      expect do
        described_class.call(export: export, actor_account: actor, destination: destination)
      end.to raise_error(IOError, 'Hosted export write incomplete')

      expect_failed_transfer(destination)
    end
  end

  def with_output_destination(filename)
    Dir.mktmpdir('hosted-export-transfer') do |output_root|
      previous_root = ENV.fetch('HOUSEHOLD_EXPORT_OUTPUT_ROOT', nil)
      ENV['HOUSEHOLD_EXPORT_OUTPUT_ROOT'] = output_root
      yield File.join(output_root, filename)
    ensure
      restore_output_root(previous_root)
    end
  end

  def expect_failed_transfer(destination)
    expect(failed_transfer_state(destination)).to eq(
      destination_exists: false,
      status: 'ready',
      downloaded_at: nil,
      downloaded_event_exists: false
    )
  end

  def failed_transfer_state(destination)
    export.reload
    {
      destination_exists: File.exist?(destination),
      status: export.status,
      downloaded_at: export.downloaded_at,
      downloaded_event_exists: SecurityAuditEvent.exists?(
        household: household,
        event_type: 'household.export.downloaded'
      )
    }
  end

  def restore_output_root(previous_root)
    if previous_root
      ENV['HOUSEHOLD_EXPORT_OUTPUT_ROOT'] = previous_root
    else
      ENV.delete('HOUSEHOLD_EXPORT_OUTPUT_ROOT')
    end
  end
end
