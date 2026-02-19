# frozen_string_literal: true

module NhsDmdStubs
  def stub_nhs_dmd_token(client_id: 'test-id', client_secret: 'test-secret', access_token: 'mock-token')
    stub_request(:post, NhsDmd::Client::TOKEN_URL)
      .with(body: {
              'grant_type' => 'client_credentials',
              'client_id' => client_id,
              'client_secret' => client_secret
            })
      .to_return(
        status: 200,
        body: { access_token: access_token, expires_in: 3600 }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
  end

  def stub_nhs_dmd_search(query:, results: [], error: nil, status: 200)
    return stub_nhs_dmd_error(query, status, error) if error

    [NhsDmd::Client::VMP_VALUE_SET, NhsDmd::Client::AMP_VALUE_SET].each do |value_set|
      stub_nhs_dmd_value_set(value_set, query, results)
    end
  end

  private

  def stub_nhs_dmd_error(query, status, error)
    stub_request(:get, %r{#{NhsDmd::Client::BASE_URL}/ValueSet/\$expand})
      .with(query: hash_including('filter' => query))
      .to_return(status: status, body: { error: error }.to_json)
  end

  def stub_nhs_dmd_value_set(value_set, query, results)
    stub_request(:get, %r{#{NhsDmd::Client::BASE_URL}/ValueSet/\$expand})
      .with(query: hash_including('url' => value_set, 'filter' => query))
      .to_return(
        status: 200,
        body: { expansion: { contains: map_results_to_fhir(results) } }.to_json,
        headers: { 'Content-Type' => 'application/json' }
      )
  end

  def map_results_to_fhir(results)
    results.select { |r| r[:system] == 'https://dmd.nhs.uk' }.map do |r|
      {
        code: r[:code],
        display: r[:display],
        system: r[:system],
        extension: [
          {
            url: 'http://hl7.org/fhir/StructureDefinition/valueset-concept-comments',
            valueString: r[:concept_class]
          }
        ]
      }
    end
  end
end

RSpec.configure do |config|
  config.include NhsDmdStubs
end
