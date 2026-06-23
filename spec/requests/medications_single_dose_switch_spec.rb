# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication single-dose switching' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  before do
    sign_in(admin)
  end

  it 'renders a validation error instead of crashing when schedules still use dosage options' do
    household = Household.find_by!(slug: default_request_household_slug)
    location = create(:location, household: household)
    medication = create(:medication, household: household, location: location, dosage_amount: nil, dosage_unit: nil)
    dosage = create(:dosage, household: household, medication: medication, amount: 10, unit: 'mg')
    create(
      :schedule,
      household: household,
      person: admin.person,
      medication: medication,
      dose_amount: dosage.amount,
      dose_unit: dosage.unit
    )

    patch medication_path(medication), params: {
      medication: {
        name: medication.name,
        location_id: medication.location_id,
        dosage_amount: 500,
        dosage_unit: 'mg',
        reorder_threshold: medication.reorder_threshold
      }
    }

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include(
      'cannot switch to a single standard dose while schedules still use dose options'
    )
  end
end
