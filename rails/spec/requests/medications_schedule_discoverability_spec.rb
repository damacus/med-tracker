# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication schedule discoverability' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'shows Add medication for someone entry point on medications index' do
    get medications_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Add medication for someone')
    expect(response.body).to include(add_medication_path)
  end

  it 'shows the schedule action on medication details' do
    medication = medications(:paracetamol)

    get medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Schedule')
    expect(response.body).to include(add_medication_path(medication_id: medication.id))
  end
end
