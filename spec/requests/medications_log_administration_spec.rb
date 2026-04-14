# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication log administration' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  before { sign_in(users(:admin)) }

  it 'renders a modal with active administration options for the medication' do
    medication = medications(:paracetamol)

    get "/medications/#{medication.id}/administration", headers: { 'Turbo-Frame' => 'modal' }

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Log administration for Paracetamol')
    expect(response.body).to include(take_medication_person_schedule_path(people(:john), schedules(:john_paracetamol)))
  end
end
