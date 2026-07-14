# frozen_string_literal: true

require 'rails_helper'
require 'zip'

RSpec.describe Households::HostedExport do
  def account(email)
    Account.create!(email: email, status: :verified)
  end

  def owner_membership(household, owner)
    household.household_memberships.create!(account: owner, role: :owner, status: :active)
  end

  let(:household) { create(:household) }
  let(:other_household) { create(:household) }
  let(:operator) { account('hosted-export-operator@example.test') }
  let(:membership) { owner_membership(household, operator) }

  before { PlatformAdmin.create!(account: operator) }

  it 'generates a durable portable export with verified household attachment bytes and checksums',
     :aggregate_failures do
    person = attach_avatar(household, 'household-avatar-bytes', 'avatar.png')
    attach_avatar(other_household, 'other-avatar-bytes', 'other.png')
    export = described_class.generate!(household:, membership:, actor_account: operator)

    expect(export).to be_ready
    verify_export_record(export, person)
    verify_export_zip(export, person)
  end

  it 'exports every portable household collection including health-event medication joins' do
    person, location, medication, event = complete_export_graph
    export = described_class.generate!(household:, membership:, actor_account: operator)
    payload = JSON.parse(export_zip_contents(export, person).fetch(:portable))

    expect(payload.fetch('scope')).to eq('household')
    expect(payload.fetch('records').keys.map(&:to_sym))
      .to match_array(PortableData::ExportRecordSerializer::COLLECTIONS.keys)
    expect(payload.dig('records', 'health_events').sole).to include(
      'portable_id' => event.portable_id,
      'medication_portable_ids' => [medication.portable_id]
    )
    expect(payload.dig('records', 'people').sole.fetch('location_portable_ids')).to include(location.portable_id)
  end

  def complete_export_graph
    person = attach_avatar(household, 'complete-export-avatar-bytes', 'complete-export-avatar.png')
    location = create(:location, household: household)
    person.location_memberships.create!(household: household, location: location)
    medication = create(:medication, household: household, location: location)
    event = HealthEvent.create!(household: household, person: person, event_kind: :suspected_side_effect,
                                title: 'Hosted export reaction', started_on: Date.current)
    HealthEventMedication.create!(household: household, health_event: event, medication: medication)
    [person, location, medication, event]
  end

  def attach_avatar(target_household, bytes, filename)
    person = create(:person, household: target_household)
    person.avatar.attach(io: StringIO.new(bytes), filename: filename, content_type: 'image/png')
    person
  end

  def verify_export_record(export, person)
    verify_attachment_manifest(export, person)
    verify_archive_isolation(export)
    expect(SecurityAuditEvent.where(household:, event_type: 'household.export.ready')).to exist
  end

  def verify_attachment_manifest(export, person)
    expect(export.artifact).to be_attached
    expect(export.manifest.fetch('attachments').sole).to include(
      'attachment_id' => person.avatar.attachment.id,
      'byte_size' => 'household-avatar-bytes'.bytesize,
      'checksum_sha256' => Digest::SHA256.hexdigest('household-avatar-bytes')
    )
  end

  def verify_archive_isolation(export)
    expect(export.artifact.download).to include('household-avatar-bytes')
    expect(export.artifact.download).not_to include('other-avatar-bytes')
    expect(export.artifact_checksum_sha256).to eq(Digest::SHA256.hexdigest(export.artifact.download))
  end

  def verify_export_zip(export, person)
    contents = export_zip_contents(export, person)
    expect(contents.fetch(:portable)).to include(person.portable_id)
    expect(contents.fetch(:manifest)).to include('medtracker.household-export.v1')
    expect(contents.fetch(:attachment)).to eq('household-avatar-bytes')
  end

  def export_zip_contents(export, person)
    contents = {}
    Zip::File.open_buffer(export.artifact.download) do |archive|
      contents.replace(
        portable: zip_entry(archive, 'portable.json'),
        manifest: zip_entry(archive, 'manifest.json'),
        attachment: zip_entry(archive, "attachments/#{person.avatar.attachment.id}.bin")
      )
    end
    contents
  end

  def zip_entry(archive, name) = archive.glob(name).sole.get_input_stream.read

  it 'records download and expiry lifecycle transitions without leaking export contents into audit metadata',
     :aggregate_failures do
    export = described_class.generate!(household:, membership:, actor_account: operator)
    artifact_blob = export.artifact.blob
    artifact_blob.attachments.load
    allow(ActiveStorage::Attachment).to receive(:exists?).and_call_original

    bytes = described_class.download!(export:, actor_account: operator)
    described_class.expire!(export:, actor_account: operator)

    expect(bytes).to be_present
    expect(export.reload).to be_expired
    expect(export.artifact).not_to be_attached
    expect(ActiveStorage::Blob.where(id: artifact_blob.id)).not_to exist
    expect(ActiveStorage::Attachment).to have_received(:exists?).with(blob_id: artifact_blob.id)
    expect(SecurityAuditEvent.where(household:, event_type: 'household.export.downloaded')).to exist
    expiry_event = SecurityAuditEvent.where(household:, event_type: 'household.export.expired').sole
    expect(expiry_event.metadata.keys).to contain_exactly('export_id', 'outcome')
  end

  it 'persists a failed lifecycle state and sanitized failure evidence' do
    allow(PortableData::Exporter).to receive(:new).and_raise(StandardError, 'sensitive failure detail')

    expect do
      described_class.generate!(household:, membership:, actor_account: operator)
    end.to raise_error(StandardError, 'sensitive failure detail')

    export = HouseholdExport.where(household:).sole
    expect(export).to be_failed
    expect(export.failure_code).to eq('StandardError')
    expect(export.attributes).not_to have_key('failure_detail')
    event = SecurityAuditEvent.where(household:, event_type: 'household.export.failed').sole
    expect(event.metadata.to_json).not_to include('sensitive failure detail')
  end

  it 'rejects an operator who is not a platform administrator' do
    expect do
      described_class.generate!(
        household:,
        membership:,
        actor_account: account('unauthorized-export-operator@example.test')
      )
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'rejects a platform administrator using another account membership outside support mode' do
    other_membership = owner_membership(household, account('unbound-export-owner@example.test'))

    expect do
      described_class.generate!(household:, membership: other_membership, actor_account: operator)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'rejects download and expiry by a requester whose household access was revoked during offboarding' do
    requester = account('former-hosted-export-owner@example.test')
    requester_membership = owner_membership(household, requester)
    export = described_class.generate!(household:, membership: requester_membership, actor_account: requester)
    Households::Offboarder.call(household: household, actor_account: operator)

    expect do
      described_class.download!(export: export, actor_account: requester)
    end.to raise_error(Pundit::NotAuthorizedError)
    expect do
      described_class.expire!(export: export, actor_account: requester)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'reauthorizes download after acquiring the household lifecycle lock' do
    export = described_class.generate!(household:, membership:, actor_account: operator)
    revoke_membership_when_household_is_locked(export, membership)

    expect do
      described_class.download!(export: export, actor_account: operator)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'reauthorizes manual expiry after acquiring the household lifecycle lock' do
    export = described_class.generate!(household:, membership:, actor_account: operator)
    revoke_membership_when_household_is_locked(export, membership)

    expect do
      described_class.expire!(export: export, actor_account: operator)
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'downloads through one stable tenant context under the forced-RLS application role' do
    export = described_class.generate!(household:, membership:, actor_account: operator)

    with_runtime_role do
      bytes = described_class.download!(export: export, actor_account: operator)
      configure_runtime_household_context

      expect(bytes).to be_present
      expect(export.reload).to be_downloaded
    end
  end

  it 'manually expires through one stable tenant context under the forced-RLS application role' do
    export = described_class.generate!(household:, membership:, actor_account: operator)

    with_runtime_role do
      described_class.expire!(export: export, actor_account: operator)
      configure_runtime_household_context

      expect(export.reload).to be_expired
      expect(export.artifact).not_to be_attached
    end
  end

  it 'refuses manual support expiry while an active retention hold exists' do
    export = described_class.generate!(household:, membership:, actor_account: operator)
    create_active_support_session
    Households::RetentionHoldManager.place!(
      household: household,
      actor_account: operator,
      reason: 'Preserve held export',
      review_on: 30.days.from_now.to_date
    )

    expect do
      described_class.expire!(export: export, actor_account: operator)
    end.to raise_error(/active retention hold/i)
    expect(export.reload).to be_ready
    expect(export.artifact).to be_attached
  end

  def revoke_membership_when_household_is_locked(export, target_membership)
    allow(export.household).to receive(:with_lock) do |&block|
      export.household.update!(status: :archived, lifecycle_state: :offboarded, offboarded_at: Time.current)
      target_membership.update!(status: :revoked, revoked_at: Time.current)
      block.call
    end
  end

  def create_active_support_session
    SupportAccessSession.create!(
      platform_admin: operator.platform_admin,
      household: household,
      reason: 'Expire hosted export',
      mfa_verified_at: Time.current,
      starts_at: 1.minute.ago,
      expires_at: 30.minutes.from_now
    )
  end

  def with_runtime_role
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
      raise ActiveRecord::Rollback
    end
  end

  def configure_runtime_household_context
    ActiveRecord::Base.connection.execute(
      "SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)"
    )
  end
end
