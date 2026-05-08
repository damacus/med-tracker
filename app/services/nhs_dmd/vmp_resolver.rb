# frozen_string_literal: true

module NhsDmd
  class VmpResolver
    def initialize(client)
      @client = client
    end

    def resolve(display, barcode_match)
      return nil unless @client.configured?
      return nil unless amp_or_ampp?(barcode_match)

      de_branded = de_brand(display)
      return nil if de_branded == display

      results = @client.search(de_branded)
      results.find { |item| item[:concept_class] == 'VMP' }
    rescue Client::ApiError, StandardError => e
      Rails.logger.error("NhsDmd::VmpResolver failed: #{e.class}: #{e.message}")
      nil
    end

    private

    def de_brand(name)
      name.to_s.sub(/\s*\([^)]*\)\z/, '').strip
    end

    def amp_or_ampp?(item)
      item[:concept_class].in?(%w[AMP AMPP])
    end
  end
end
