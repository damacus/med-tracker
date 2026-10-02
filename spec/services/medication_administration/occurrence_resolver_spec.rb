require 'rails_helper'

RSpec.describe MedicationAdministration::OccurrenceResolver do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules

  let(:source) { schedules(:john_movicol) }
  let(:membership) do
    actor = Account.create!(email: 'outcome-recorder@example.test', status: :verified)
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

  before { grant }

  def key_for(date = Date.current)
    projection = MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: date, end_date: date
    )
    projection.call.first.key
  end

  def not_taken(**attributes)
    resolver.call(key: key_for, action: 'not_taken', reason: 'unwell', **attributes)
  end

  it 'records attributable not-taken without creating an administration or changing stock' do
    source.medication.update!(current_supply: 100)
    stock = source.medication.current_supply
    result = nil
    expect { result = not_taken }.not_to change(MedicationTake, :count)

    expect(result).to have_attributes(outcome: 'not_taken', reason: 'unwell', resolved_by_membership: membership)
    expect(source.medication.reload.current_supply).to eq(stock)
  end

  it 'returns an identical retry without another row or audit version' do
    first = not_taken

    expect { expect(not_taken).to eq(first) }.not_to change(PaperTrail::Version, :count)
    expect(source.medication_dose_occurrences.count).to eq(1)
  end

  it 'rejects a competing reason without changing the committed outcome' do
    first = not_taken

    expect { not_taken(reason: 'refused') }.to raise_error(described_class::Error) do |error|
      expect(error.code).to eq('already_resolved')
    end
    expect(first.reload.reason).to eq('unwell')
  end

  it 'rejects a future occurrence without persisting anything' do
    future_key = key_for(Date.tomorrow)

    expect do
      resolver.call(key: future_key, action: 'not_taken')
    end.to raise_error(described_class::Error, 'Occurrence is not due')
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'accepts an overdue occurrence inside the schedule range' do
    source.update!(start_date: Date.yesterday)

    result = resolver.call(key: key_for(Date.yesterday), action: 'not_taken')

    expect(result).to have_attributes(window_starts_on: Date.yesterday, outcome: 'not_taken')
  end

  it 'rolls back invalid context and its audit changes' do
    expect do
      expect { not_taken(reason: 'invalid') }.to raise_error(ActiveRecord::RecordInvalid)
    end.not_to change(PaperTrail::Version, :count)
    expect(source.medication_dose_occurrences.reload).to be_empty
  end

  it 'requires current record access even when the client queued with manage access' do
    grant.update!(access_level: :view)

    expect { not_taken }.to raise_error(Pundit::NotAuthorizedError)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'rechecks a membership deactivated after the authorization context was built' do
    original_context = authorization
    HouseholdMembership.find(membership.id).update!(status: :revoked)

    expect do
      described_class.new(source: source, authorization: original_context).call(key: key_for, action: 'not_taken')
    end.to raise_error(Pundit::NotAuthorizedError)
  end

  it 'requires manage access to reopen a not-taken outcome' do
    saved = not_taken
    grant.update!(access_level: :record)

    expect do
      resolver.call(key: key_for, action: 'reopen', if_match: Api::RecordEtag.for(saved))
    end.to raise_error(Pundit::NotAuthorizedError)
    expect(saved.reload.outcome).to eq('not_taken')
  end

  it 'reopens with a current ETag and retains the prior context in audit history' do
    saved = not_taken

    result = resolver.call(key: key_for, action: 'reopen', if_match: Api::RecordEtag.for(saved))

    expect(result).to have_attributes(outcome: 'open', reason: nil, note: nil, resolved_at: nil)
    expect(result.versions.last.reify.reason).to eq('unwell')
  end

  it 'rejects missing and stale correction preconditions' do
    saved = not_taken
    expect { resolver.call(key: key_for, action: 'reopen') }.to raise_error(described_class::Error)
    expect do
      resolver.call(key: key_for, action: 'reopen', if_match: 'stale')
    end.to raise_error(described_class::Error)
    expect(saved.reload.outcome).to eq('not_taken')
  end

  it 'rejects a valid key for a different source without exposing its details' do
    other_source = schedules(:jane_ibuprofen)
    other_key = MedicationAdministration::OccurrenceProjection.new(
      source: other_source, start_date: Date.current, end_date: Date.current
    ).call.first.key

    expect { resolver.call(key: other_key, action: 'not_taken') }
      .to raise_error(described_class::Error, 'Occurrence is unavailable')
  end

  it 'rejects a previously valid key when recurrence changes before resolution' do
    original_key = key_for
    source.update!(start_date: Date.tomorrow)

    expect { resolver.call(key: original_key, action: 'not_taken') }
      .to raise_error(described_class::Error, 'Occurrence is unavailable')
  end

  it 'does not resolve a reopened historical row after its recurrence is removed' do
    saved = not_taken
    original_key = key_for
    resolver.call(key: original_key, action: 'reopen', if_match: Api::RecordEtag.for(saved))
    source.update!(start_date: Date.tomorrow)

    expect { resolver.call(key: original_key, action: 'not_taken') }
      .to raise_error(described_class::Error, 'Occurrence is unavailable')
  end

  it 'refuses not-taken when an existing administration already satisfies the occurrence' do
    MedicationTake.create!(schedule: source, taken_at: Time.current, dose_amount: 1, dose_unit: 'sachet')

    expect { not_taken }.to raise_error(described_class::Error, 'Occurrence is already resolved')
    expect(source.medication_dose_occurrences).to be_empty
  end
end
