require 'json'
require 'uri'
require 'webmock'

WebMock.enable!
WebMock.disable_net_connect!(allow_localhost: true, allow: 'mail-test')

token_url = 'https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token'
WebMock.stub_request(:post, token_url).to_return(
  status: 200,
  body: { access_token: 'contract-nhs-token', expires_in: 3600 }.to_json,
  headers: { 'Content-Type' => 'application/json' }
)

upstream_items = [
  ['contract-upstream-250', 'Contractupstream 250mg/5ml oral suspension'],
  ['contract-upstream-500', 'Contractupstream 500mg tablets'],
  ['contract-upstream-125', 'Contractupstream 125mg/5ml oral suspension']
].map do |code, display|
  {
    code: code,
    display: display,
    system: 'https://dmd.nhs.uk',
    extension: [{ url: 'http://hl7.org/fhir/StructureDefinition/valueset-concept-comments', valueString: 'AMPP' }]
  }
end

WebMock.stub_request(:get, %r{https://ontology\.nhs\.uk/production1/fhir/ValueSet/\$expand}).to_return do |request|
  parameters = URI.decode_www_form(request.uri.query.to_s).to_h
  if parameters['filter'] == 'contractupstream'
    contains = parameters['url'] == 'https://dmd.nhs.uk/ValueSet/VMP' ? upstream_items : []
    { status: 200, body: { expansion: { contains: contains } }.to_json,
      headers: { 'Content-Type' => 'application/json' } }
  else
    { status: 503, body: { error: 'Contract upstream unavailable' }.to_json,
      headers: { 'Content-Type' => 'application/json' } }
  end
end
