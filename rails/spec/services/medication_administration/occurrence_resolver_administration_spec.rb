require 'rails_helper'

RSpec.describe MedicationAdministration::OccurrenceResolver do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules

  let(:source) { schedules(:john_movicol) }
  let(:membership) do
    actor = Account.create!(email: 'outcome-administration@example.test', status: :verified)
    source.household.household_memberships.create!(account: actor, role: :member, status: :active)
  end
  let(:authorization) do
    AuthorizationContext.new(account: membership.account, household: source.household, membership: membership)
  end
  let(:resolver) { described_class.new(source: source, authorization: authorization) }

  before do
    PersonAccessGrant.create!(household: source.household, household_membership: membership,
                              person: source.person, access_level: :manage, relationship_type: :family_member)
    source.medication.update!(current_supply: 100)
  end

  def occurrence_key
    MedicationAdministration::OccurrenceProjection.new(
      source: source.reload, start_date: Date.current, end_date: Date.current
    ).call.first.key
  end

  def record_take(**attributes)
    resolver.take(key: occurrence_key, client_uuid: 'f8d86aa0-bf86-4e1f-a08a-ffed32506724', **attributes)
  end

  it 'links one canonical administration and decrements tracked stock once' do
    result = nil
    expect { result = record_take }.to change(MedicationTake, :count).by(1)

    expect(result.outcome).to eq('taken')
    expect(result.medication_take.source).to eq(source)
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'returns the linked take on an identical client UUID retry' do
    first = record_take

    expect { expect(record_take).to eq(first) }.not_to change(MedicationTake, :count)
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'rejects a competing take without another administration' do
    record_take

    expect { record_take(client_uuid: SecureRandom.uuid) }
      .to raise_error(described_class::Error, 'Occurrence is already resolved')
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'replaces not-taken while retaining its audited context' do
    prior = resolver.call(key: occurrence_key, action: 'not_taken', reason: 'refused', note: 'Initially declined')

    result = record_take

    expect(result.id).to eq(prior.id)
    expect(result).to have_attributes(outcome: 'taken', reason: nil, note: nil)
    expect(result.versions.last.reify.note).to eq('Initially declined')
    expect(source.medication.reload.current_supply).to eq(99)
  end

  it 'does not change not-taken when the dose recorder rejects stock availability' do
    prior = resolver.call(key: occurrence_key, action: 'not_taken', reason: 'medicine_unavailable')
    source.medication.update!(current_supply: 0)

    expect { record_take }.to raise_error(described_class::Error)
    expect(prior.reload.outcome).to eq('not_taken')
    expect(MedicationTake.where(schedule: source)).to be_empty
  end

  it 'rejects a take timestamp outside the identified occurrence window' do
    expect { record_take(taken_at: 1.day.ago) }
      .to raise_error(described_class::Error, 'Dose does not match the occurrence window')
    expect(source.medication.reload.current_supply).to eq(100)
    expect(source.medication_dose_occurrences).to be_empty
  end

  it 'refuses to reopen a linked administration' do
    saved = record_take

    expect { resolver.call(key: occurrence_key, action: 'reopen', if_match: Api::RecordEtag.for(saved)) }
      .to raise_error(described_class::Error, 'Occurrence cannot be reopened')
    expect(saved.reload.outcome).to eq('taken')
  end

  it 'requires record access before administering a dose' do
    PersonAccessGrant.active.find_by!(household_membership: membership, person: source.person)
                     .update!(access_level: :view)

    expect { record_take }.to raise_error(Pundit::NotAuthorizedError)
    expect(source.medication.reload.current_supply).to eq(100)
  end

  it 'rolls the administration and stock back when the outcome cannot be saved' do
    allow(resolver).to receive(:link_take).and_raise(ActiveRecord::RecordInvalid)

    expect { record_take }.to raise_error(ActiveRecord::RecordInvalid)
    expect(source.medication.reload.current_supply).to eq(100)
    expect(source.medication_takes.reload).to be_empty
    expect(source.medication_dose_occurrences).to be_empty
  end
end
