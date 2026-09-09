require 'rails_helper'

RSpec.describe MedicationAdministration::OccurrenceResolver do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :person_medications

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c), max_daily_doses: 1)
  end
  let(:membership) do
    actor = Account.create!(email: 'routine-outcome-recorder@example.test', status: :verified)
    source.household.household_memberships.create!(account: actor, role: :member, status: :active)
  end
  let(:authorization) do
    AuthorizationContext.new(account: membership.account, household: source.household, membership: membership)
  end
  let(:grant) do
    PersonAccessGrant.create!(household: source.household, household_membership: membership,
                              person: source.person, access_level: :manage, relationship_type: :family_member)
  end
  let(:resolver) { described_class.new(source: source, authorization: authorization) }

  before do
    travel_to(Time.zone.local(2026, 9, 9, 12))
    grant
    source.update!(created_at: 2.months.ago)
    source.medication.update!(current_supply: 100)
  end

  def key_for(date = Date.current)
    MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: date, end_date: date
    ).call.first.key
  end

  def not_taken
    resolver.call(key: key_for, action: 'not_taken', reason: 'unwell')
  end

  it 'records and replays a routine decision without a take, stock change or duplicate audit' do
    first = nil
    expect { first = not_taken }.not_to change(MedicationTake, :count)
    expect(first).to have_attributes(outcome: 'not_taken', person_medication: source,
                                     resolved_by_membership: membership)
    expect { expect(not_taken).to eq(first) }.not_to change(PaperTrail::Version, :count)
    expect(source.medication.reload.current_supply).to eq(100)
  end

  %w[daily weekly monthly].each do |cycle|
    it "links and replays a take within the #{cycle} window" do
      source.update!(dose_cycle: cycle)
      uuid = SecureRandom.uuid
      result = resolver.take(key: key_for, taken_at: Time.current, client_uuid: uuid)
      expect(result).to have_attributes(outcome: 'taken', person_medication: source)
      expect(result.medication_take.source).to eq(source)
      expect do
        resolver.take(key: key_for, taken_at: Time.current, client_uuid: uuid)
      end.not_to change(MedicationTake, :count)
      expect(source.medication.reload.current_supply).to eq(99)
    end
  end

  it 'reopens a routine outcome with the current version and retains its audit history' do
    record = not_taken
    result = resolver.call(key: key_for, action: 'reopen', if_match: Api::RecordEtag.for(record))
    expect(result).to have_attributes(outcome: 'open', reason: nil)
    expect(result.versions.last.reify.reason).to eq('unwell')
  end

  it 'requires the current version when an external client replaces not-taken' do
    record = not_taken
    expect { resolver.take(key: key_for, client_uuid: SecureRandom.uuid, if_match: nil) }
      .to raise_error(described_class::Error) { |error| expect(error.code).to eq('precondition_required') }
    result = resolver.take(key: key_for, client_uuid: SecureRandom.uuid, if_match: Api::RecordEtag.for(record))
    expect(result).to be_taken
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'rejects a timestamp outside the cycle before deducting stock' do
    source.update!(dose_cycle: :monthly)
    expect do
      resolver.take(key: key_for, taken_at: Date.current.prev_month.in_time_zone, client_uuid: SecureRandom.uuid)
    end.to raise_error(described_class::Error, 'Dose does not match the occurrence window')
    expect(source.medication.reload.current_supply).to eq(100)
  end

  it 'corrects a saved monthly outcome within its original window after a cycle change' do
    source.update!(dose_cycle: :monthly)
    key = key_for
    record = not_taken
    source.update!(dose_cycle: :daily)

    result = resolver.take(key: key, taken_at: Time.current, client_uuid: SecureRandom.uuid,
                           if_match: Api::RecordEtag.for(record))

    expect(result).to be_taken
    expect(result.window_ends_on).to eq(Date.current.end_of_month)
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'rejects a take before assignment creation even inside the same cycle' do
    source.update!(dose_cycle: :monthly, created_at: 1.day.ago)
    expect do
      resolver.take(key: key_for, taken_at: 2.days.ago, client_uuid: SecureRandom.uuid)
    end.to raise_error(described_class::Error, 'Dose does not match the occurrence window')
    expect(source.medication.reload.current_supply).to eq(100)
  end

  it 'requires record access for resolution and manage access for reopening' do
    record = not_taken
    grant.update!(access_level: :record)
    expect { resolver.call(key: key_for, action: 'reopen', if_match: Api::RecordEtag.for(record)) }
      .to raise_error(Pundit::NotAuthorizedError)
    grant.update!(access_level: :view)
    expect { not_taken }.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'rejects a valid key belonging to another routine source' do
    other = person_medications(:jane_vitamin_d)
    key = MedicationAdministration::OccurrenceProjection.verifier.generate(
      ['person_medication', other.portable_id, Date.current.iso8601, 1]
    )
    expect { resolver.call(key: key, action: 'not_taken') }
      .to raise_error(described_class::Error, 'Occurrence is unavailable')
  end

  it 'rejects a saved key after the source becomes as-needed' do
    key = key_for
    source.update!(administration_kind: :as_needed)
    expect { resolver.call(key: key, action: 'not_taken') }
      .to raise_error(described_class::Error, 'Occurrence is unavailable')
  end

  it 'rolls back the take and stock if the outcome cannot be saved' do
    allow(resolver).to receive(:link_take).and_raise(ActiveRecord::RecordInvalid)
    expect { resolver.take(key: key_for, client_uuid: SecureRandom.uuid) }.to raise_error(ActiveRecord::RecordInvalid)
    expect(source.medication.reload.current_supply).to eq(100)
    expect(source.medication_takes.reload).to be_empty
  end
end
