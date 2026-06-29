# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationStockSourceResolver do
  fixtures :accounts, :people, :users

  let(:taken_at) { Time.current }
  let(:location) { create(:location, household: fixture_household) }
  let(:medication) do
    create(:medication,
           name: 'Paracetamol',
           dosage_amount: 500,
           dosage_unit: 'mg',
           location: location,
           household: fixture_household,
           current_supply: 20,
           supply_at_last_restock: 20)
  end

  before { FixtureHouseholdSetup.apply! }

  # Build a source double pointing at `medication`
  def build_source(med, can_take: true, paused: false)
    instance_double(
      PersonMedication,
      medication_id: med.id,
      medication: med,
      can_take_at?: can_take,
      paused?: paused
    )
  end

  describe '#available_medications' do
    context 'when user is nil (unscoped — returns source medication only)' do
      it 'returns the source medication when it has stock' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.available_medications).to include(medication)
      end

      it 'excludes medications that are out of stock' do
        medication.update!(current_supply: 0)
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.available_medications).to be_empty
      end
    end
  end

  describe '#blocked_reason' do
    context 'when medication is out of stock' do
      before { medication.update!(current_supply: 0) }

      it 'returns :out_of_stock' do
        source = build_source(medication, can_take: true)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.blocked_reason).to eq(:out_of_stock)
      end
    end

    context 'when the source is paused' do
      it 'returns :paused' do
        source = build_source(medication, paused: true)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.blocked_reason).to eq(:paused)
      end
    end

    context 'when medication is in stock but cooldown prevents dose' do
      it 'returns :cooldown' do
        source = build_source(medication, can_take: false)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.blocked_reason).to eq(:cooldown)
      end
    end

    context 'when medication is in stock and cooldown allows dose' do
      it 'returns nil' do
        source = build_source(medication, can_take: true)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.blocked_reason).to be_nil
      end
    end
  end

  describe '#resolve_selected' do
    context 'when taken_from_medication_id is blank and only one available medication' do
      it 'returns the only available medication' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.resolve_selected(nil)).to eq(medication)
      end
    end

    context 'when taken_from_medication_id is provided and matches an in-stock medication' do
      it 'returns the selected medication' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.resolve_selected(medication.id)).to eq(medication)
      end
    end

    context 'when taken_from_medication_id matches an out-of-stock medication' do
      before { medication.update!(current_supply: 0) }

      it 'returns nil' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.resolve_selected(medication.id)).to be_nil
      end
    end

    context 'when taken_from_medication_id does not match any medication' do
      it 'returns nil' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.resolve_selected(-1)).to be_nil
      end
    end

    context 'when taken_from_medication_id is a string' do
      it 'coerces to integer and returns the medication' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.resolve_selected(medication.id.to_s)).to eq(medication)
      end
    end
  end

  describe '#selection_required?' do
    context 'when only one available medication and id is blank' do
      it 'returns false' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.selection_required?(nil)).to be false
      end
    end

    context 'when id is already provided' do
      it 'returns false' do
        source = build_source(medication)
        resolver = described_class.new(user: nil, source: source, taken_at: taken_at)

        expect(resolver.selection_required?(medication.id)).to be false
      end
    end

    context 'when there are multiple available matching medications' do
      let(:location2) { create(:location, household: fixture_household) }
      let!(:second_medication) do
        create(:medication,
               name: medication.name,
               dosage_amount: medication.dosage_amount,
               dosage_unit: medication.dosage_unit,
               location: location2,
               household: fixture_household,
               current_supply: 10,
               supply_at_last_restock: 10)
      end

      it 'returns true when no id provided (admin user sees all scope)' do
        # john is an admin — MedicationPolicy::Scope returns scope.all
        # second_medication is in scope alongside the primary medication
        expect(second_medication).to be_persisted
        source = build_source(medication)
        resolver = described_class.new(user: users(:john), source: source, taken_at: taken_at)

        expect(resolver.selection_required?(nil)).to be true
      end
    end
  end

  def fixture_household
    users(:john).person.account.first_active_household_membership.household
  end
end
