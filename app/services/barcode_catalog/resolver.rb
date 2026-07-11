# frozen_string_literal: true

module BarcodeCatalog
  class Resolver
    Result = Data.define(:status, :barcode, :match, :source, :error) do
      def resolved?
        status == :resolved
      end

      def error?
        status == :error
      end
    end

    def initialize(lookup: Lookup.new)
      @lookup = lookup
    end

    def call(value)
      barcode = NhsDmdBarcode.normalize_gtin(value)
      return outcome(:invalid, barcode:, error: 'invalid_barcode') unless NhsDmd::BarcodeLookup.barcode_query?(barcode)

      match = lookup.lookup(barcode)
      return outcome(:not_found, barcode:) unless match

      outcome(:resolved, barcode:, match:, source: match[:source])
    rescue StandardError => e
      Rails.logger.error("BarcodeCatalog::Resolver failed: #{e.class}: #{e.message}")
      outcome(:error, barcode:, error: 'barcode_resolution_failed')
    end

    private

    attr_reader :lookup

    def outcome(status, barcode:, match: nil, source: nil, error: nil)
      Result.new(status:, barcode:, match:, source:, error:)
    end
  end
end
