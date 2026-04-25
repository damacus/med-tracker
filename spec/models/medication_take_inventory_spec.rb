# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTake do
  let(:person) { create(:person) }
  let(:medication) do
    create(
      :medication,
      name: 'Lisinopril',
      dosage_amount: 10,
      dosage_unit: 'mg',
      current_supply: 10,
      reorder_threshold: 2
    )
  end

  it 'decrements the matching alternate dosage record instead of an arbitrary row' do
    schedule = create_ambiguous_schedule(person: person, medication: medication)
    alternate_medication, inventory_options = create_alternate_medication_with_ambiguous_options(
      assigned_medication: medication,
      location_name: 'Deterministic Alt'
    )

    create_taken_from_schedule(schedule: schedule, taken_from_medication: alternate_medication)

    expect(inventory_options[:morning].reload.current_supply).to eq(6)
    expect(inventory_options[:evening].reload.current_supply).to eq(3)
    expect(alternate_medication.reload.current_supply).to eq(9)
  end

  it 'syncs aggregate inventory once when decrementing a dosage record' do
    schedule = create_ambiguous_schedule(person: person, medication: medication)
    alternate_medication, = create_alternate_medication_with_ambiguous_options(
      assigned_medication: medication,
      location_name: 'Single Sync Alt'
    )

    allow(alternate_medication).to receive(:sync_inventory_from_dosage_records!).and_call_original

    create_taken_from_schedule(schedule: schedule, taken_from_medication: alternate_medication)

    expect(alternate_medication).to have_received(:sync_inventory_from_dosage_records!).once
  end

  it 'locks the dosage record before locking aggregate inventory' do
    schedule = create_ambiguous_schedule(person: person, medication: medication)
    alternate_medication, inventory_options = create_alternate_medication_with_ambiguous_options(
      assigned_medication: medication,
      location_name: 'Lock Order Alt'
    )
    lock_order = record_lock_order(inventory: alternate_medication, dosage_option: inventory_options[:evening])

    build_taken_from_schedule(schedule: schedule, taken_from_medication: alternate_medication).send(
      :decrement_dosage_option_stock,
      alternate_medication,
      inventory_options[:evening]
    )

    expect(lock_order).to eq(%i[dosage inventory])
  end

  it 'rejects an alternate medication that does not have the selected dosage record' do
    schedule = create_ambiguous_schedule(person: person, medication: medication)
    alternate_medication = create_incomplete_alternate_medication(assigned_medication: medication)

    take = build_taken_from_schedule(schedule: schedule, taken_from_medication: alternate_medication)

    expect(take).not_to be_valid
    expect(take.errors[:taken_from_medication]).to include('must include stock for the selected dose')
  end

  def create_ambiguous_schedule(person:, medication:)
    source_options = create_ambiguous_tracked_options(medication: medication, current_supply: 10, reorder_threshold: 2)

    create(
      :schedule,
      person: person,
      medication: medication,
      dosage: source_options[:evening]
    )
  end

  def create_ambiguous_tracked_options(medication:, current_supply:, reorder_threshold:)
    create_option_pair(
      medication: medication,
      supplies: { morning: current_supply, evening: current_supply },
      reorder_threshold: reorder_threshold
    )
  end

  def create_alternate_medication_with_ambiguous_options(assigned_medication:, location_name:)
    alternate_medication = create_matching_medication(
      medication: assigned_medication,
      location: create(:location, name: location_name),
      current_supply: 10,
      reorder_threshold: 2
    )

    [
      alternate_medication,
      create_option_pair(
        medication: alternate_medication,
        supplies: { morning: 6, evening: 4 },
        reorder_threshold: 1
      )
    ]
  end

  def create_incomplete_alternate_medication(assigned_medication:)
    alternate_medication = create_matching_medication(
      medication: assigned_medication,
      location: create(:location, name: 'Incomplete Alt'),
      current_supply: 10,
      reorder_threshold: 2
    )
    create_tracked_dosage(
      medication: alternate_medication,
      frequency: 'Morning',
      description: 'Morning tablet',
      current_supply: 6,
      reorder_threshold: 1
    )

    alternate_medication
  end

  def record_lock_order(inventory:, dosage_option:)
    lock_order = []
    record_lock(dosage_option, lock_order, :dosage)
    record_lock(inventory, lock_order, :inventory)
    lock_order
  end

  def record_lock(record, lock_order, label)
    allow(record).to receive(:with_lock).and_wrap_original do |method, *args, &block|
      lock_order << label
      method.call(*args, &block)
    end
  end

  def create_matching_medication(medication:, location:, **overrides)
    create(
      :medication,
      {
        name: medication.name,
        location: location,
        dosage_amount: medication.dosage_amount,
        dosage_unit: medication.dosage_unit,
        current_supply: 12,
        reorder_threshold: 2
      }.merge(overrides)
    )
  end

  def create_option_pair(medication:, supplies:, reorder_threshold:)
    supplies.to_h do |frequency, current_supply|
      [
        frequency,
        create_tracked_dosage(
          medication: medication,
          frequency: frequency.to_s.capitalize,
          description: "#{frequency.to_s.capitalize} tablet",
          current_supply: current_supply,
          reorder_threshold: reorder_threshold
        )
      ]
    end
  end

  def create_tracked_dosage(medication:, frequency:, description:, current_supply:, reorder_threshold:)
    create(
      :dosage,
      medication: medication,
      amount: 1,
      unit: 'tablet',
      frequency: frequency,
      description: description,
      current_supply: current_supply,
      reorder_threshold: reorder_threshold
    )
  end

  def create_taken_from_schedule(schedule:, taken_from_medication:)
    described_class.create!(
      schedule: schedule,
      taken_at: Time.current,
      amount_ml: 10.0,
      taken_from_medication: taken_from_medication,
      taken_from_location: taken_from_medication.location
    )
  end

  def build_taken_from_schedule(schedule:, taken_from_medication:)
    described_class.new(
      schedule: schedule,
      taken_at: Time.current,
      amount_ml: 10.0,
      taken_from_medication: taken_from_medication,
      taken_from_location: taken_from_medication.location
    )
  end
end
