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

  def restore_output_root(previous_root)
    if previous_root
      ENV['HOUSEHOLD_EXPORT_OUTPUT_ROOT'] = previous_root
    else
      ENV.delete('HOUSEHOLD_EXPORT_OUTPUT_ROOT')
    end
  end
end
