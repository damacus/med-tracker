# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication wizard dose option follow-up' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  it 'creates the medication, primary dose option, and first schedule from the wizard' do
    sign_in(users(:admin))

    post medications_path,
         params: {
           wizard: 'true',
           onboarding_schedule: {
             person_id: people(:john).id,
             schedule_type: 'multiple_daily',
             frequency: 'Twice daily',
             start_date: Time.zone.today.to_s,
             end_date: 1.month.from_now.to_date.to_s,
             max_daily_doses: '2',
             min_hours_between_doses: '12',
             dose_cycle: 'daily',
             schedule_config: JSON.generate(
               schedule_type: 'multiple_daily',
               frequency: 'Twice daily',
               times: %w[08:00 20:00]
             )
           },
           medication: {
             name: 'Wizard Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '2.5',
                 unit: 'ml',
                 frequency: 'Twice daily',
                 default_for_adults: '0',
                 default_for_children: '1',
                 default_max_daily_doses: '2',
                 default_min_hours_between_doses: '12',
                 default_dose_cycle: 'daily'
               }
             }
           }
         },
         headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

    expect(response).to have_http_status(:ok)
    expect(response.content_type).to include('text/vnd.turbo-stream.html')

    medication = Medication.order(:id).last
    dosage = medication.dosage_records.order(:id).last
    schedule = Schedule.order(:id).last
    body = response.body

    expect(body).to include('turbo-stream')
    expect(body).to include('target="wizard-content"')
    expect(body).to include('Manage dose options')
    expect(body).to include(edit_medication_path(medication, return_to: medication_path(medication)))
    expect(body).not_to include('target="dosage-form"')
    expect(body).not_to include('target="dosage-list"')
    expect(medication.current_supply).to eq(10)
    expect(medication.reorder_threshold).to eq(1)
    expect(medication.dosage_records.count).to eq(1)
    expect(dosage).to have_attributes(
      amount: BigDecimal('2.5'),
      unit: 'ml',
      frequency: 'Twice daily',
      default_for_adults: false,
      default_for_children: true,
      default_max_daily_doses: 2,
      default_min_hours_between_doses: BigDecimal('12.0')
    )
    expect(dosage.default_dose_cycle).to eq('daily')
    expect(schedule).to have_attributes(
      person: people(:john),
      medication: medication,
      source_dosage_option: dosage,
      dose_amount: BigDecimal('2.5'),
      dose_unit: 'ml',
      frequency: 'Twice daily',
      max_daily_doses: 2,
      min_hours_between_doses: 12,
      start_date: Time.zone.today,
      end_date: 1.month.from_now.to_date
    )
    expect(schedule.dose_cycle).to eq('daily')
    expect(schedule.schedule_type).to eq('multiple_daily')
    expect(schedule.schedule_config).to include(
      'schedule_type' => 'multiple_daily',
      'frequency' => 'Twice daily',
      'times' => %w[08:00 20:00]
    )
  end

  it 'creates the initial child schedule from the edited primary dose when suggested adult doses are present' do
    sign_in(users(:admin))

    post medications_path,
         params: {
           wizard: 'true',
           onboarding_schedule: {
             person_id: people(:child_patient).id,
             schedule_type: 'daily',
             frequency: 'Once daily',
             start_date: Time.zone.today.to_s,
             end_date: 1.month.from_now.to_date.to_s,
             max_daily_doses: '1',
             min_hours_between_doses: '24',
             dose_cycle: 'daily',
             schedule_config: JSON.generate(
               schedule_type: 'daily',
               frequency: 'Once daily',
               times: %w[08:00]
             )
           },
           medication: {
             name: 'Child Wizard Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '5',
                 unit: 'ml',
                 frequency: 'Once daily',
                 default_for_adults: '0',
                 default_for_children: '1',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               },
               '1' => {
                 amount: '500',
                 unit: 'mg',
                 frequency: 'Once daily',
                 default_for_adults: '1',
                 default_for_children: '0',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               }
             }
           }
         },
         headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

    medication = Medication.order(:id).last
    child_dosage = medication.dosage_records.find_by!(amount: BigDecimal('5'), unit: 'ml')
    schedule = Schedule.order(:id).last

    expect(schedule).to have_attributes(
      person: people(:child_patient),
      source_dosage_option: child_dosage,
      dose_amount: BigDecimal('5'),
      dose_unit: 'ml'
    )
  end

  it 'rolls back medication creation when the onboarding schedule person is unauthorized' do
    sign_in(users(:jane))

    medication_count = Medication.count
    schedule_count = Schedule.count

    post medications_path,
         params: {
           wizard: 'true',
           onboarding_schedule: {
             person_id: people(:john).id,
             schedule_type: 'daily',
             frequency: 'Once daily',
             start_date: Time.zone.today.to_s,
             end_date: 1.month.from_now.to_date.to_s,
             max_daily_doses: '1',
             min_hours_between_doses: '24',
             dose_cycle: 'daily',
             schedule_config: JSON.generate(times: ['08:00'])
           },
           medication: {
             name: 'Unauthorized Person Schedule Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '5',
                 unit: 'ml',
                 frequency: 'Once daily',
                 default_for_adults: '1',
                 default_for_children: '0',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               }
             }
           }
         }

    expect(response).to have_http_status(:not_found)
    expect(Medication.count).to eq(medication_count)
    expect(Schedule.count).to eq(schedule_count)
  end

  it 'rolls back medication creation when the onboarding schedule person does not exist' do
    sign_in(users(:admin))

    medication_count = Medication.count
    schedule_count = Schedule.count

    post medications_path,
         params: {
           wizard: 'true',
           onboarding_schedule: {
             person_id: Person.maximum(:id) + 1,
             schedule_type: 'daily',
             frequency: 'Once daily',
             start_date: Time.zone.today.to_s,
             end_date: 1.month.from_now.to_date.to_s,
             max_daily_doses: '1',
             min_hours_between_doses: '24',
             dose_cycle: 'daily',
             schedule_config: JSON.generate(times: ['08:00'])
           },
           medication: {
             name: 'Invalid Person Schedule Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '5',
                 unit: 'ml',
                 frequency: 'Once daily',
                 default_for_adults: '1',
                 default_for_children: '0',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               }
             }
           }
         }

    expect(response).to have_http_status(:not_found)
    expect(Medication.count).to eq(medication_count)
    expect(Schedule.count).to eq(schedule_count)
  end

  it 'rolls back medication creation when the onboarding schedule is invalid' do
    sign_in(users(:admin))

    expect do
      post medications_path,
           params: {
             wizard: 'true',
             onboarding_schedule: {
               person_id: people(:john).id,
               schedule_type: 'daily',
               frequency: 'Once daily',
               start_date: Time.zone.today.to_s,
               end_date: 1.day.ago.to_date.to_s,
               max_daily_doses: '1',
               min_hours_between_doses: '24',
               dose_cycle: 'daily',
               schedule_config: {
                 times: ['08:00']
               }
             },
             medication: {
               name: 'Invalid Schedule Medication',
               category: 'Vitamin',
               current_supply: '10',
               reorder_threshold: '1',
               location_id: locations(:home).id,
               dosage_records_attributes: {
                 '0' => {
                   amount: '5',
                   unit: 'ml',
                   frequency: 'Once daily',
                   default_for_adults: '1',
                   default_for_children: '0',
                   default_max_daily_doses: '1',
                   default_min_hours_between_doses: '24',
                   default_dose_cycle: 'daily'
                 }
               }
             }
           }
    end.not_to change(Medication, :count)

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('must be after the start date')
  end

  it 'falls back to the medication page for non-turbo requests' do
    sign_in(users(:admin))

    post medications_path,
         params: {
           wizard: 'true',
           onboarding_schedule: {
             person_id: people(:john).id,
             schedule_type: 'daily',
             frequency: 'As directed',
             start_date: Time.zone.today.to_s,
             end_date: 1.month.from_now.to_date.to_s,
             max_daily_doses: '1',
             min_hours_between_doses: '24',
             dose_cycle: 'daily',
             schedule_config: {
               times: []
             }
           },
           medication: {
             name: 'Redirected Wizard Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '5',
                 unit: 'ml',
                 frequency: 'As directed',
                 default_for_adults: '1',
                 default_for_children: '0',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               }
             }
           }
         }

    expect(response).to redirect_to(medication_path(Medication.order(:id).last))
  end
end
