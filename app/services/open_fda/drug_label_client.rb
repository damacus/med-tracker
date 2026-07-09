# frozen_string_literal: true

require 'net/http'

module OpenFda
  class DrugLabelClient
    ENDPOINT = 'https://api.fda.gov/drug/label.json'

    def labels(limit:)
      response = Net::HTTP.get_response(request_uri(limit))
      raise "openFDA label request failed with HTTP #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body).fetch('results')
    end

    private

    def request_uri(limit)
      uri = URI(ENDPOINT)
      uri.query = URI.encode_www_form(search: '_exists_:drug_interactions', limit: Integer(limit))
      uri
    end
  end
end
