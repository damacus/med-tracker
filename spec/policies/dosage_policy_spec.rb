# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DosagePolicy, type: :policy do
  let(:owner) { household_policy_member(role: :owner) }
  let(:household) { owner.fetch(:household) }
  let(:member) { household_policy_member(role: :member, household: household) }
  let(:person) { household_policy_person(household) }
  let(:medication) { household_policy_medication(household) }
  let(:dosage) do
    MedicationDosageOption.create!(
      medication: medication,
      amount: 500,
      unit: 'mg',
      frequency: 'Daily',
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )
  end

  it 'mirrors medication visibility for reads' do
    expect(described_class.new(member.fetch(:context), dosage).show?).to be(false)

    household_policy_schedule(household, person: person, medication: medication)
    grant_policy_person_access(member, person, access_level: :view)

    expect(described_class.new(member.fetch(:context), dosage).show?).to be(true)
  end

  it 'mirrors medication manager permissions for writes' do
    expect(described_class.new(owner.fetch(:context), dosage).create?).to be(true)
    expect(described_class.new(owner.fetch(:context), dosage).update?).to be(true)
    expect(described_class.new(owner.fetch(:context), dosage).destroy?).to be(true)
    expect(described_class.new(member.fetch(:context), dosage).update?).to be(false)
  end
end
