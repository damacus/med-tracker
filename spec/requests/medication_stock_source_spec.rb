# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication stock sources' do
  fixtures :accounts, :people, :locations, :medications, :users, :dosages, :location_memberships

  let(:admin) { users(:admin) }
  let(:person) { household_person(people(:jane)) }
  let(:source_medication) { household_medication(medications(:ibuprofen)) }
  let(:school_location) { household_location(locations(:school)) }

  before do
    sign_in(admin)
    MedicationTake.delete_all
  end

  describe 'POST /people/:person_id/schedules/:id/take_medication' do
    it 'deducts stock from the selected alternate medication location' do
      schedule = build_schedule
      alternate_medication = build_alternate_medication

      expect do
        post take_medication_person_schedule_path(person, schedule),
             params: { taken_from_medication_id: alternate_medication.id }
      end.to change(MedicationTake, :count).by(1)

      take = MedicationTake.order(:id).last
      expect(take.taken_from_medication).to eq(alternate_medication)
      expect(take.inventory_location).to eq(school_location)
      expect(source_medication.reload.current_supply).to eq(30)
      expect(alternate_medication.reload.current_supply).to eq(6)
    end

    it 'keeps the selected alternate source when recording a historical dose' do
      schedule = build_schedule
      alternate_medication = build_alternate_medication
      submitted_time = Time.zone.local(2026, 4, 27, 7, 45)

      travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
        expect do
          post take_medication_person_schedule_path(person, schedule),
               params: {
                 medication_take: {
                   taken_at: submitted_time.strftime('%Y-%m-%dT%H:%M'),
                   taken_from_medication_id: alternate_medication.id
                 }
               }
        end.to change(MedicationTake, :count).by(1)
      end

      take = MedicationTake.order(:id).last
      expect(take.taken_at).to be_within(1.second).of(submitted_time)
      expect(take.taken_from_medication).to eq(alternate_medication)
      expect(take.inventory_location).to eq(school_location)
      expect(alternate_medication.reload.current_supply).to eq(6)
    end

    it 'rejects a selected source with a different dosage' do
      schedule = build_schedule
      incompatible_medication = build_incompatible_medication

      expect do
        post take_medication_person_schedule_path(person, schedule),
             params: { medication_take: { taken_from_medication_id: incompatible_medication.id } }
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(person_path(person))
      expect(flash[:alert]).to include('Selected location is unavailable')
    end

    it 'requires an explicit location selection when multiple stock sources are available' do
      build_alternate_medication
      schedule = build_schedule

      expect do
        post take_medication_person_schedule_path(person, schedule)
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(person_path(person))
      expect(flash[:alert]).to include('Choose a location')
    end
  end

  describe 'POST /people/:person_id/person_medications/:id/take_medication' do
    it 'records the selected alternate location on PRN takes' do
      person_medication = build_person_medication
      alternate_medication = build_alternate_medication

      expect do
        post take_medication_person_person_medication_path(person, person_medication),
             params: { taken_from_medication_id: alternate_medication.id }
      end.to change(MedicationTake, :count).by(1)

      take = MedicationTake.order(:id).last
      expect(take.taken_from_medication).to eq(alternate_medication)
      expect(take.inventory_location).to eq(school_location)
      expect(alternate_medication.reload.current_supply).to eq(6)
    end

    it 'keeps the selected alternate source when recording a historical PRN dose' do
      person_medication = build_person_medication
      alternate_medication = build_alternate_medication
      submitted_time = Time.zone.local(2026, 4, 27, 10, 5)

      travel_to(Time.zone.local(2026, 4, 28, 12, 0)) do
        expect do
          post take_medication_person_person_medication_path(person, person_medication),
               params: {
                 medication_take: {
                   taken_at: submitted_time.strftime('%Y-%m-%dT%H:%M'),
                   taken_from_medication_id: alternate_medication.id
                 }
               }
        end.to change(MedicationTake, :count).by(1)
      end

      take = MedicationTake.order(:id).last
      expect(take.taken_at).to be_within(1.second).of(submitted_time)
      expect(take.taken_from_medication).to eq(alternate_medication)
      expect(take.inventory_location).to eq(school_location)
      expect(alternate_medication.reload.current_supply).to eq(6)
    end

    it 'rejects a selected PRN source with a different dosage' do
      person_medication = build_person_medication
      incompatible_medication = build_incompatible_medication

      expect do
        post take_medication_person_person_medication_path(person, person_medication),
             params: { medication_take: { taken_from_medication_id: incompatible_medication.id } }
      end.not_to change(MedicationTake, :count)

      expect(response).to redirect_to(person_path(person))
      expect(flash[:alert]).to include('Selected location is unavailable')
    end
  end

  describe 'POST /schedules/:schedule_id/medication_takes' do
    it 'accepts taken_from_medication_id via medication_take params' do
      schedule = build_schedule
      alternate_medication = build_alternate_medication

      expect do
        post schedule_medication_takes_path(schedule),
             params: { medication_take: { taken_from_medication_id: alternate_medication.id } }
      end.to change(MedicationTake, :count).by(1)

      expect(response).to redirect_to(root_path)
      expect(MedicationTake.order(:id).last.taken_from_medication).to eq(alternate_medication)
    end
  end

  def build_schedule
    Schedule.create!(
      person: person,
      medication: source_medication,
      dosage: dosages(:ibuprofen_adult),
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
  end

  def build_person_medication
    PersonMedication.create!(
      person: person,
      medication: source_medication,
      dose_amount: source_medication.dose_amount,
      dose_unit: source_medication.dose_unit
    )
  end

  def build_alternate_medication
    Medication.create!(
      household: person.household,
      name: source_medication.name,
      location: school_location,
      category: source_medication.category,
      dose_amount: source_medication.dose_amount,
      dose_unit: source_medication.dose_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end

  def build_incompatible_medication
    Medication.create!(
      household: person.household,
      name: source_medication.name,
      location: school_location,
      category: source_medication.category,
      dose_amount: source_medication.dose_amount + 100,
      dose_unit: source_medication.dose_unit,
      current_supply: 7,
      reorder_threshold: 1
    )
  end
end
