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
    it 'creates a PRN schedule from medication metadata and the selected predefined dose' do
      medication = medications(:paracetamol)
      dosage = dosages(:paracetamol_child)
      person_medication_count = PersonMedication.count

      expect do
        post person_medication_assignments_path(person),
             params: {
               medication_assignment: {
                 medication_id: medication.id,
                 source_dosage_option_id: dosage.id
               }
             }
      end.to change(Schedule, :count).by(1)

      expect(PersonMedication.count).to eq(person_medication_count)
      expect(response).to redirect_to(person_path(person))
      schedule = Schedule.order(:id).last

      expect(schedule).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: dosage,
        dose_amount: BigDecimal('250.0'),
        dose_unit: 'mg',
        frequency: 'Every 4-6 hours',
        max_daily_doses: 4,
        min_hours_between_doses: 4,
        start_date: Time.zone.today,
        end_date: 1.month.from_now.to_date
      )
      expect(schedule.dose_cycle).to eq('daily')
      expect(schedule.schedule_type).to eq('prn')
      expect(schedule.schedule_config).to include('as_needed' => true)
    end

    it 'creates a non-PRN schedule when medication metadata is not as needed' do
      medication = medications(:ibuprofen)
      dosage = dosages(:ibuprofen_child)

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
      expect(schedule).to have_attributes(
        dose_amount: BigDecimal('200.0'),
        dose_unit: 'mg',
        frequency: 'Every 6-8 hours',
        max_daily_doses: 4,
        min_hours_between_doses: 6
      )
    end

    it 'ignores submitted timing overrides and uses the selected dose defaults' do
      medication = medications(:ibuprofen)
      dosage = dosages(:ibuprofen_child)

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

      schedule = Schedule.order(:id).last

      expect(schedule.max_daily_doses).to eq(4)
      expect(schedule.min_hours_between_doses).to eq(6)
      expect(schedule.dose_cycle).to eq('daily')
    end

    it 'rejects a forged inaccessible medication id' do
      foreign_medication = Medication.create!(
        name: 'Foreign Household Medication',
        location: locations(:grandmas),
        category: 'Analgesic',
        dosage_amount: 250,
        dosage_unit: 'mg',
        current_supply: 10,
        reorder_threshold: 1
      )

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
