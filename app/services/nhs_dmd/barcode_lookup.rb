# frozen_string_literal: true

module NhsDmd
  class BarcodeLookup
    CACHE_PREFIX = 'nhs_dmd/barcode'

    def lookup(barcode)
      barcode_candidates(barcode).filter_map { |candidate| fetch(candidate) }.first
    end

    def self.expire(gtin)
      normalized = NhsDmdBarcode.normalize_gtin(gtin)
      candidates_for(normalized).each do |candidate|
        Rails.cache.delete(cache_key(candidate))
      end
    end

    def self.cache_key(gtin)
      "#{CACHE_PREFIX}/#{gtin}"
    end

    def self.candidates_for(barcode)
      normalized = NhsDmdBarcode.normalize_gtin(barcode)
      return [] if normalized.blank?

      candidates = [normalized]
      candidates << normalized.rjust(14, '0') if normalized.length == 13
      candidates << normalized.delete_prefix('0') if normalized.length == 14 && normalized.start_with?('0')
      candidates.uniq
    end

    private

    def barcode_candidates(barcode)
      self.class.candidates_for(barcode)
    end

    def fetch(gtin)
      Rails.cache.fetch(self.class.cache_key(gtin), expires_in: 12.hours) do
        record = NhsDmdBarcode.find_by(gtin: gtin)
        next nil unless record

        {
          code: record.code,
          display: record.display,
          system: record.system,
          concept_class: record.concept_class
        }
      end
    end
  end
end
