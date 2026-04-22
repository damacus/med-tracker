# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication NHS guidance' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'renders NHS patient guidance on the medication details page when available' do
    medication = medications(:paracetamol)
    guidance = Struct.new(:title, :description, :webpage, :last_reviewed_on, :sections, :author_name, :author_url,
                          keyword_init: true).new(
                            title: 'Paracetamol for adults',
                            description: 'Find out how paracetamol treats pain and high temperature.',
                            webpage: 'https://www.nhs.uk/medicines/paracetamol-for-adults/',
                            last_reviewed_on: Date.new(2024, 9, 12),
                            sections: [
                              Struct.new(:title, :text, keyword_init: true).new(
                                title: 'How and when to take it',
                                text: 'Take paracetamol only as directed on the packet or by a clinician.'
                              )
                            ],
                            author_name: 'NHS website',
                            author_url: 'https://www.nhs.uk'
                          )
    service = instance_double(NhsWebsiteContent::MedicineGuidanceLookup, call: guidance)

    allow(NhsWebsiteContent::MedicineGuidanceLookup).to receive(:new).and_return(service)

    get medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('NHS medicine guidance')
    expect(response.body).to include('Paracetamol for adults')
    expect(response.body).to include('Take paracetamol only as directed on the packet or by a clinician.')
    expect(response.body).to include('View full NHS guidance')
    expect(response.body).to include('https://www.nhs.uk/medicines/paracetamol-for-adults/')
  end
end
