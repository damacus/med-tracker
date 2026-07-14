# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HostedRestore::RuntimeVerifier do
  let(:first_household) { create(:household) }
  let(:second_household) { create(:household) }
  let(:runtime_image_path) { Rails.root.join('tmp/spec-runtime-image-ref') }

  before do
    runtime_image_path.write("app:v1\n")
    prepare_sample(first_household, 'first')
    prepare_sample(second_household, 'second')
  end

  after do
    ActiveStorage::Attachment.where(household_id: [first_household.id, second_household.id]).find_each(&:purge)
    FileUtils.rm_f(runtime_image_path)
  end

  it 'proves forced RLS default denial and two-household clinical, audit, attachment, and storage isolation' do
    result = with_runtime_role do
      described_class.new(
        household_ids: [first_household.id, second_household.id], runtime_image_path:
      ).call
    end

    expect(result).to include(
      database_role: 'med_tracker_app', app_image: 'app:v1', forced_rls: true, default_deny: true,
      isolation: { clinical: true, audit: true, attachments: true },
      storage: { samples_verified: 2 }
    )
    expected_schema_version = ActiveRecord::Base.connection.select_value(
      'SELECT max(version) FROM schema_migrations'
    ).to_s
    expect(result.fetch(:schema_version)).to eq(expected_schema_version)
  end

  it 'fails closed when the application role or two distinct complete samples are absent' do
    expect do
      described_class.new(
        household_ids: [first_household.id, second_household.id], runtime_image_path:
      ).call
    end.to raise_error(HostedRestore::VerificationError, 'runtime_role_required')

    ActiveStorage::Attachment.where(household_id: second_household.id).find_each(&:purge)
    expect do
      with_runtime_role do
        described_class.new(
          household_ids: [first_household.id, second_household.id], runtime_image_path:
        ).call
      end
    end.to raise_error(HostedRestore::VerificationError, 'representative_sample_missing')
  end

  it 'reads image provenance from the baked file and ignores runtime environment overrides' do
    ENV['RUNTIME_APP_IMAGE'] = 'attacker.example/med-tracker:forged'

    result = with_runtime_role do
      described_class.new(
        household_ids: [first_household.id, second_household.id], runtime_image_path:
      ).call
    end

    expect(result.fetch(:app_image)).to eq('app:v1')
  ensure
    ENV.delete('RUNTIME_APP_IMAGE')
  end

  it 'fails closed when baked image provenance is missing or invalid' do
    FileUtils.rm_f(runtime_image_path)
    expect do
      with_runtime_role do
        described_class.new(
          household_ids: [first_household.id, second_household.id], runtime_image_path:
        ).call
      end
    end.to raise_error(HostedRestore::VerificationError, 'runtime_app_image_invalid')
  end

  def prepare_sample(household, label)
    person = create(:person, household:)
    person.avatar.attach(io: StringIO.new("#{label}-restored-object"), filename: "#{label}.png",
                         content_type: 'image/png')
    Audit::Event.record!(household:, event_type: "hosted_restore.#{label}", metadata: { outcome: 'success' })
  end

  def with_runtime_role(&)
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      yield
    end
  end
end
