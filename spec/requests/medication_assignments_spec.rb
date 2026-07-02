# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication assignments' do
  fixtures :accounts, :people, :locations, :location_memberships, :medications, :users, :carer_relationships, :dosages

  let(:person) { people(:child_user_person) }
  let(:parent) { users(:parent) }

  before { sign_in(parent) }

  describe 'GET /people/:person_id/medication_assignments/new' do
    it 'renders the unified assignment workflow without the type chooser' do
      get new_person_medication_assignment_path(person), headers: { 'Turbo-Frame' => 'modal' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Add Medication for')
      expect(response.body).to include('Choose a medication')
      expect(response.body).to include('Choose the dose')
      expect(response.body).to include('Review')
      expect(response.body).not_to include('Prescribed / Scheduled')
      expect(response.body).not_to include('How is this medication taken?')
    end
  end

  describe 'POST /people/:person_id/medication_assignments' do
    it 'creates an as-needed person medication from PRN medication metadata and the selected predefined dose' do
      medication = medications(:paracetamol)
      dosage = dosages(:paracetamol_child)
      schedule_count = Schedule.count

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: dosage.id
               }
             }
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      expect(response).to redirect_to(person_path(person))
      person_medication = PersonMedication.order(:id).last

      expect(person_medication).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: dosage,
        dose_amount: BigDecimal('250.0'),
        dose_unit: 'mg',
        max_daily_doses: 4,
        min_hours_between_doses: 4
      )
      expect(person_medication.dose_cycle).to eq('daily')
      expect(person_medication.administration_kind).to eq('as_needed')
    end

    it 'creates an as-needed person medication from ibuprofen PRN metadata' do
      medication = medications(:ibuprofen)
      dosage = dosages(:ibuprofen_child)
      medication.update!(default_schedule_type: :prn)
      schedule_count = Schedule.count

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: dosage.id
               }
             }
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      person_medication = PersonMedication.order(:id).last

      expect(person_medication).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: dosage,
        dose_amount: BigDecimal('200.0'),
        dose_unit: 'mg',
        max_daily_doses: 4,
        min_hours_between_doses: 6
      )
      expect(person_medication.dose_cycle).to eq('daily')
      expect(person_medication.administration_kind).to eq('as_needed')
    end

    it 'creates a non-PRN schedule when medication metadata is not as needed' do
      medication = medications(:ibuprofen)
      dosage = dosages(:ibuprofen_child)
      medication.update!(
        default_schedule_type: :multiple_daily,
        default_schedule_config: {
          'schedule_type' => 'multiple_daily',
          'frequency' => 'Twice daily',
          'times' => %w[07:15 19:45]
        }
      )

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: dosage.id
               }
             }
      end.to change(Schedule, :count).by(1)

      schedule = Schedule.order(:id).last

      expect(schedule.schedule_type).to eq('multiple_daily')
      expect(schedule.schedule_config).not_to include('as_needed' => true)
      expect(schedule.schedule_config).to include(
        'schedule_type' => 'multiple_daily',
        'frequency' => 'Every 6-8 hours',
        'times' => %w[07:15 19:45]
      )
      expect(schedule).to have_attributes(
        dose_amount: BigDecimal('200.0'),
        dose_unit: 'mg',
        frequency: 'Every 6-8 hours',
        max_daily_doses: 4,
        min_hours_between_doses: 6
      )
    end

    it 'creates a routine person medication for a vitamin from a matching legacy dose fallback' do
      medication = medications(:vitamin_c)
      schedule_count = Schedule.count

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 dose_amount: '500.0',
                 dose_unit: 'mg'
               }
             }
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      person_medication = PersonMedication.order(:id).last

      expect(person_medication).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: nil,
        dose_amount: BigDecimal('500.0'),
        dose_unit: 'mg'
      )
      expect(person_medication.administration_kind).to eq('routine')
    end

    it 'creates a routine person medication for a vitamin with a selected predefined dose' do
      medication = medications(:vitamin_d)
      dosage = dosages(:vitamin_d_daily)
      schedule_count = Schedule.count

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: dosage.id
               }
             }
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      person_medication = PersonMedication.order(:id).last

      expect(person_medication).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: dosage,
        dose_amount: BigDecimal('1000.0'),
        dose_unit: 'IU',
        max_daily_doses: 1
      )
      expect(person_medication.dose_cycle).to eq('daily')
      expect(person_medication.administration_kind).to eq('routine')
    end

    it 'ignores submitted timing overrides and uses the selected dose defaults' do
      medication = medications(:ibuprofen)
      dosage = dosages(:ibuprofen_child)
      medication.update!(default_schedule_type: :prn)

      post person_medication_assignments_path(person),
           params: {
             medication_assignment: {
               medication_id: medication.id,
               source_dosage_option_id: dosage.id,
               max_daily_doses: '99',
               min_hours_between_doses: '0',
               dose_cycle: 'monthly'
             }
           }

      person_medication = PersonMedication.order(:id).last

      expect(person_medication.max_daily_doses).to eq(4)
      expect(person_medication.min_hours_between_doses).to eq(6)
      expect(person_medication.dose_cycle).to eq('daily')
    end

    it 'rejects a forged inaccessible medication id' do
      foreign_medication = create_foreign_medication

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: foreign_medication.id
               }
             }
      end.not_to change(Schedule, :count)

      expect(response).to redirect_to(root_path)
    end

    def create_foreign_medication
      household = Household.create!(name: 'Foreign Assignment Household', slug: 'foreign-assignment-household')
      location = household.locations.create!(name: 'Foreign Assignment Location')
      Medication.create!(
        household: household,
        name: 'Foreign Household Medication',
        location: location,
        category: 'Analgesic',
        dose_amount: 250,
        dose_unit: 'mg',
        current_supply: 10,
        reorder_threshold: 1
      )
    end

    it 'rejects a forged dose option from another medication' do
      medication = medications(:ibuprofen)
      forged_dosage = dosages(:paracetamol_child)

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: forged_dosage.id
               }
             }
      end.not_to change(Schedule, :count)

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Select a valid predefined dose')
    end
  end
end
