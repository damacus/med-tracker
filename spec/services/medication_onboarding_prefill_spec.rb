# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationOnboardingPrefill do
  def expect_calpol_six_plus_doses(dosages)
    expect(dosages).to include(*calpol_six_plus_dose_expectations)
  end

  def calpol_six_plus_dose_expectations
    [
      dose_expectation(5, 'Children 6-8 years'),
      dose_expectation(7.5, 'Children 8-10 years'),
      dose_expectation(10, 'Children 10-12 years'),
      dose_expectation(10, 'Children 12-16 years'),
      dose_expectation(15, 'Children 12-16 years'),
      dose_expectation(10, 'Adults and children over 16 years'),
      dose_expectation(20, 'Adults and children over 16 years')
    ]
  end

  def dose_expectation(amount, description)
    a_hash_including(
      amount: amount,
      unit: 'ml',
      description: description,
      default_max_daily_doses: 4,
      default_min_hours_between_doses: 4
    )
  end

  def wellman_open_food_facts_lookup
    instance_double(OpenFoodFacts::BarcodeLookup, lookup: wellman_open_food_facts_result)
  end

  def wellman_open_food_facts_result
    {
      name: 'Wellman Original',
      description: 'Daily multivitamin food supplement',
      category: 'Supplement',
      package_quantity: 30,
      package_unit: 'tablet'
    }
  end

  def expect_wellman_prefill(result)
    expect(result.medication_attributes).to include(
      name: 'Wellman Original',
      description: 'Daily multivitamin food supplement',
      category: 'Supplement',
      dosage_amount: 1,
      dosage_unit: 'tablet',
      current_supply: 30,
      reorder_threshold: 7
    )
  end

  describe '#call' do
    it 'returns curated combo-pack defaults for Pregnacare Plus' do
      result = described_class.new.call(
        barcode: '5021265232062',
        code: '35394411000001103',
        name: 'Pregnacare Plus tablets and capsules (Vitabiotics Ltd)'
      )

      expect(result.medication_attributes).to include(
        current_supply: 84,
        reorder_threshold: 21
      )
      expect(result.dosage_records_attributes).to contain_exactly(
        a_hash_including(amount: 1, unit: 'tablet', current_supply: 56, reorder_threshold: 14),
        a_hash_including(amount: 1, unit: 'capsule', current_supply: 28, reorder_threshold: 7)
      )
    end

    it 'derives safe single-form defaults from parseable dm+d display text' do
      result = described_class.new.call(
        name: 'Paracetamol 500mg tablets (Acme Ltd) 16 tablet'
      )

      expect(result.medication_attributes).to include(
        dosage_amount: 1,
        dosage_unit: 'tablet',
        current_supply: 16,
        reorder_threshold: 4
      )
      expect(result.dosage_records_attributes).to contain_exactly(
        a_hash_including(amount: 1, unit: 'tablet', current_supply: 16, reorder_threshold: 4)
      )
    end

    it 'returns curated refill defaults for Calpol vapour pads' do
      result = described_class.new.call(
        barcode: '3574661646435',
        name: 'Calpol Vapour Plug & Nightlight + 3 Refill Pads'
      )

      expect(result.medication_attributes).to include(
        dosage_amount: 1,
        dosage_unit: 'pad',
        current_supply: 3,
        reorder_threshold: 0
      )
      expect(result.dosage_records_attributes).to contain_exactly(
        a_hash_including(amount: 1, unit: 'pad', current_supply: 3, reorder_threshold: 0)
      )
    end

    it 'returns curated onboarding defaults for Calpol Six Plus oral suspension' do
      result = described_class.new.call(
        code: '316811000001106',
        name: 'Calpol Six Plus 250mg/5ml oral suspension (McNeil Products Ltd)'
      )

      expect(result.medication_attributes).to include(
        category: 'Analgesic',
        description: a_string_including('mild to moderate pain'),
        warnings: a_string_including('Contains paracetamol'),
        dosage_unit: 'ml',
        reorder_threshold: 0
      )
      expect_calpol_six_plus_doses(result.dosage_records_attributes)
    end

    it 'uses explicit pack metadata without polluting the medication name' do
      result = described_class.new.call(
        name: 'Wellman Original',
        package_quantity: '29',
        package_unit: 'tablet'
      )

      expect(result.medication_attributes).to include(
        dosage_amount: 1,
        dosage_unit: 'tablet',
        current_supply: 29,
        reorder_threshold: 7
      )
      expect(result.dosage_records_attributes).to contain_exactly(
        a_hash_including(amount: 1, unit: 'tablet', current_supply: 29, reorder_threshold: 7)
      )
    end

    it 'leaves supply and dosage blank when pack metadata has no usable unit' do
      result = described_class.new.call(
        name: 'Wellman Original',
        package_quantity: '29',
        package_unit: nil
      )

      expect(result.medication_attributes).not_to include(:dosage_amount, :dosage_unit, :current_supply)
      expect(result.dosage_records_attributes).to eq([])
    end

    it 'looks up Open Food Facts supplement metadata from a barcode' do
      open_food_facts_lookup = wellman_open_food_facts_lookup

      result = described_class.new(open_food_facts_lookup: open_food_facts_lookup).call(
        barcode: '5021265221301'
      )

      expect(open_food_facts_lookup).to have_received(:lookup).with('5021265221301')
      expect_wellman_prefill(result)
      expect(result.dosage_records_attributes).to contain_exactly(
        a_hash_including(amount: 1, unit: 'tablet', current_supply: 30, reorder_threshold: 7)
      )
    end

    it 'does not look up Open Food Facts for a dm+d-coded medication' do
      open_food_facts_lookup = instance_double(OpenFoodFacts::BarcodeLookup, lookup: nil)

      described_class.new(open_food_facts_lookup: open_food_facts_lookup).call(
        barcode: '5016298210989',
        code: '13629411000001105',
        name: 'Laxido Orange oral powder sachets (Galen Ltd)'
      )

      expect(open_food_facts_lookup).not_to have_received(:lookup)
    end
  end
end
