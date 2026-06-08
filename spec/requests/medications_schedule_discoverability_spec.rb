# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication schedule discoverability' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules

  before { sign_in(users(:admin)) }

  it 'shows Add Schedule entry point on medications index' do
    get medications_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Add Schedule')
    expect(response.body).to include(add_medication_path)
  end

  it 'shows Add Schedule entry point on medication details page' do
    medication = medications(:paracetamol)

    get medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Schedule')
    expect(response.body).to include(
      schedules_workflow_path(medication_id: medication.id, return_to: medication_path(medication))
    )
  end

  it 'lists all schedules for the medication and excludes other medications' do
    medication = medications(:paracetamol)
    future_schedule = Schedule.create!(
      person: people(:john),
      medication: medication,
      dose_amount: 1000,
      dose_unit: 'mg',
      frequency: 'Future vitamin course',
      start_date: 1.month.from_now.to_date,
      end_date: 2.months.from_now.to_date
    )
    stopped_schedule = Schedule.create!(
      person: people(:john),
      medication: medication,
      dose_amount: 1000,
      dose_unit: 'mg',
      frequency: 'Stopped supplier course',
      start_date: Time.zone.today,
      end_date: 1.year.from_now.to_date,
      stopped_on: Time.zone.today
    )

    get medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(schedules(:john_paracetamol).frequency)
    expect(response.body).to include(future_schedule.frequency)
    expect(response.body).to include(stopped_schedule.frequency)
    expect(response.body).not_to include(schedules(:jane_ibuprofen).frequency)
  end
end
