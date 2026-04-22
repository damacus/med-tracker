# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsWebsiteContent::MedicineGuidanceLookup do
  subject(:lookup) { described_class.new(client: client) }

  let(:client) { instance_double(NhsWebsiteContent::Client) }
  let(:index_response) do
    {
      'significantLink' => [
        {
          'name' => 'Paracetamol for adults',
          'description' => 'Brand names include Panadol. Find out how paracetamol treats pain and high temperature.',
          'url' => 'https://api.service.nhs.uk/nhs-website-content/medicines/paracetamol-for-adults/'
        },
        {
          'name' => 'Phenoxymethylpenicillin',
          'description' => 'Find out how phenoxymethylpenicillin treats bacterial infections.',
          'url' => 'https://api.service.nhs.uk/nhs-website-content/medicines/phenoxymethylpenicillin/'
        }
      ],
      'relatedLink' => []
    }
  end
  let(:detail_response) do
    {
      'name' => 'Paracetamol for adults',
      'description' => 'Find out how paracetamol for adults treats aches, pains and high temperature.',
      'webpage' => 'https://www.nhs.uk/medicines/paracetamol-for-adults/',
      'hasPart' => [
        {
          'headline' => 'How and when to take it',
          'healthAspect' => 'UsageOrScheduleHealthAspect',
          'text' => 'Take paracetamol only as directed on the packet or by a clinician.'
        },
        {
          'headline' => 'Side effects',
          'healthAspect' => 'SideEffectsHealthAspect',
          'text' => 'Side effects are rare if you stay within the recommended dose.'
        }
      ],
      'mainEntityOfPage' => {
        'lastReviewed' => ['2024-09-12T00:00:00+00:00']
      },
      'author' => {
        'name' => 'NHS website',
        'url' => 'https://www.nhs.uk',
        'logo' => 'https://assets.nhs.uk/logo.png'
      }
    }
  end

  before do
    allow(client).to receive(:configured?).and_return(true)
    allow(client).to receive(:list_medicines).with(category: 'P', page: '1').and_return(index_response)
    allow(client).to receive(:get_medicine)
      .with(slug: 'paracetamol-for-adults', modules: true)
      .and_return(detail_response)
  end

  it 'matches branded medication names to NHS medicine guidance and extracts key sections' do
    result = lookup.call('Panadol 500mg tablets')

    expect(result.title).to eq('Paracetamol for adults')
    expect(result.description).to include('paracetamol for adults')
    expect(result.webpage).to eq('https://www.nhs.uk/medicines/paracetamol-for-adults/')
    expect(result.last_reviewed_on).to eq(Date.new(2024, 9, 12))
    expect(result.sections.map { |section| [section.title, section.text] }).to eq(
      [
        ['How and when to take it', 'Take paracetamol only as directed on the packet or by a clinician.'],
        ['Side effects', 'Side effects are rare if you stay within the recommended dose.']
      ]
    )
  end

  it 'returns nil when the client is unavailable' do
    allow(client).to receive(:configured?).and_return(false)

    expect(lookup.call('Paracetamol')).to be_nil
  end
end
