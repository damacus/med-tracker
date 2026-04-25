# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication NHS guidance' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'loads the medication page without calling the NHS guidance lookup' do
    medication = medications(:paracetamol)

    allow(NhsWebsiteContent::MedicineGuidanceLookup).to receive(:new)

    get medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include("src=\"/medications/#{medication.id}/nhs_guidance\"")
    expect(response.body).not_to include('NHS medicine guidance')
    expect(NhsWebsiteContent::MedicineGuidanceLookup).not_to have_received(:new)
  end

  it 'renders NHS patient guidance in the lazy guidance frame when available' do
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
    service = instance_double(NhsWebsiteContent::MedicineGuidanceLookup)

    allow(NhsWebsiteContent::MedicineGuidanceLookup).to receive(:new).and_return(service)
    allow(service).to receive(:call).with(medication.name).and_return(guidance)

    get nhs_guidance_medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('NHS medicine guidance')
    expect(response.body).to include('Paracetamol for adults')
    expect(response.body).to include('Take paracetamol only as directed on the packet or by a clinician.')
    expect(response.body).to include('View full NHS guidance')
    expect(response.body).to include('https://www.nhs.uk/medicines/paracetamol-for-adults/')
  end

  it 'returns an empty matching frame when no NHS guidance is available' do
    medication = medications(:paracetamol)
    service = instance_double(NhsWebsiteContent::MedicineGuidanceLookup, call: nil)

    allow(NhsWebsiteContent::MedicineGuidanceLookup).to receive(:new).and_return(service)

    get nhs_guidance_medication_path(medication)

    expect(response).to have_http_status(:ok)
    expect(response.body).to include("id=\"medication_#{medication.id}_nhs_guidance\"")
    expect(response.body).not_to include('NHS medicine guidance')
  end
end
