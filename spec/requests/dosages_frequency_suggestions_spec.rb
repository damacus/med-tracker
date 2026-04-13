# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication dose option suggestions' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages

  before { sign_in(users(:admin)) }

  it 'renders frequency suggestion badges on the medication edit form' do
    medication = medications(:paracetamol)

    get edit_medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Once daily')
    expect(response.body).to include('Every 4–6 hours')
    expect(response.body).to include('Every morning')
    expect(response.body).to include('As needed (PRN)')
    expect(response.body).to include('name="medication[dosage_records_attributes][0][frequency]"')
    expect(response.body).to match(/data-controller="[^"]*frequency-suggestions[^"]*"/)
    expect(response.body).to include('data-action="click->frequency-suggestions#suggest"')
  end
end
