# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::Client do
  subject(:client) { described_class.new }

  let(:base_url) { 'https://ontology.nhs.uk/production1/fhir' }
  let(:vmp_url) { 'https://dmd.nhs.uk/ValueSet/VMP' }
  let(:amp_url) { 'https://dmd.nhs.uk/ValueSet/AMP' }

  describe '#search' do
    context 'when the API returns results' do
      let(:fhir_response) do
        {
          'resourceType' => 'ValueSet',
          'expansion' => {
            'total' => 2,
            'contains' => [
              {
                'system' => 'https://dmd.nhs.uk',
                'code' => '39720311000001101',
                'display' => 'Aspirin 300mg tablets',
                'extension' => [
                  {
                    'url' => 'http://hl7.org/fhir/StructureDefinition/valueset-concept-comments',
                    'valueString' => 'VMP'
                  }
                ]
              },
              {
                'system' => 'https://dmd.nhs.uk',
                'code' => '39720411000001102',
                'display' => 'Aspirin 75mg tablets',
                'extension' => []
              }
            ]
          }
        }.to_json
      end

      before do
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 200, body: fhir_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'returns an array of hashes with code, display, and concept_class' do
        results = client.search('aspirin')

        expect(results).to be_an(Array)
        expect(results.length).to eq(2)
        expect(results.first).to include(
          code: '39720311000001101',
          display: 'Aspirin 300mg tablets'
        )
      end

      it 'includes the concept_class when present in extensions' do
        results = client.search('aspirin')

        expect(results.first[:concept_class]).to eq('VMP')
      end

      it 'sets concept_class to nil when extension is absent' do
        results = client.search('aspirin')

        expect(results.last[:concept_class]).to be_nil
      end
    end

    context 'when the API returns no results' do
      let(:empty_response) do
        {
          'resourceType' => 'ValueSet',
          'expansion' => {
            'total' => 0,
            'contains' => []
          }
        }.to_json
      end

      before do
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 200, body: empty_response, headers: { 'Content-Type' => 'application/json' })
      end

      it 'returns an empty array' do
        results = client.search('nonexistentmedicine12345')

        expect(results).to eq([])
      end
    end

    context 'when the API returns no expansion key' do
      before do
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 200, body: '{"resourceType":"ValueSet"}',
                     headers: { 'Content-Type' => 'application/json' })
      end

      it 'returns an empty array' do
        results = client.search('aspirin')

        expect(results).to eq([])
      end
    end

    context 'when the API returns a non-200 status' do
      before do
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 503, body: 'Service Unavailable')
      end

      it 'raises NhsDmd::Client::ApiError' do
        expect { client.search('aspirin') }.to raise_error(NhsDmd::Client::ApiError)
      end
    end

    context 'when the request times out' do
      before do
        stub_request(:get, /ontology\.nhs\.uk/).to_timeout
      end

      it 'raises NhsDmd::Client::ApiError' do
        expect { client.search('aspirin') }.to raise_error(NhsDmd::Client::ApiError)
      end
    end

    context 'when the query is blank' do
      it 'returns an empty array without making an HTTP request' do
        results = client.search('')

        expect(results).to eq([])
        expect(WebMock).not_to have_requested(:get, /ontology\.nhs\.uk/)
      end
    end

    context 'when authenticated with credentials' do
      before do
        allow(ENV).to receive(:fetch).and_call_original
        allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_ID', nil).and_return('test-client-id')
        allow(ENV).to receive(:fetch).with('NHS_DMD_CLIENT_SECRET', nil).and_return('test-secret')

        stub_request(:post, %r{openid-connect/token})
          .to_return(status: 200, body: { 'access_token' => 'test-token' }.to_json,
                     headers: { 'Content-Type' => 'application/json' })
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 200, body: '{"resourceType":"ValueSet","expansion":{"total":0,"contains":[]}}',
                     headers: { 'Content-Type' => 'application/json' })
      end

      it 'includes an Authorization header in the request' do
        client.search('aspirin')

        expect(WebMock).to have_requested(:get, /ontology\.nhs\.uk/)
          .with(headers: { 'Authorization' => 'Bearer test-token' }).twice
      end
    end

    context 'with a custom count' do
      before do
        stub_request(:get, /ontology\.nhs\.uk/)
          .to_return(status: 200, body: '{"resourceType":"ValueSet","expansion":{"total":0,"contains":[]}}',
                     headers: { 'Content-Type' => 'application/json' })
      end

      it 'passes the count parameter to both VMP and AMP API requests' do
        client.search('aspirin', count: 5)

        expect(WebMock).to have_requested(:get, /ontology\.nhs\.uk/)
          .with(query: hash_including('count' => '5')).twice
      end
    end
  end
end
