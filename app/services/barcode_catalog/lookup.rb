# frozen_string_literal: true

module BarcodeCatalog
  class Lookup
    def lookup(barcode)
      barcode_candidates(barcode).each do |candidate|
        external = lookup_external(candidate)
        return external if external

        local = lookup_local(candidate)
        return local if local
      end

      nil
    end

    private

    def barcode_candidates(barcode)
      NhsDmd::BarcodeLookup.candidates_for(barcode)
    end

    def lookup_external(candidate)
      record = BarcodeCatalogEntry.find_by(gtin: candidate)
      return nil unless record

      {
        code: record.code,
        display: record.display,
        system: record.system,
        concept_class: record.concept_class,
        source: record.source
      }
    end

    def lookup_local(candidate)
      record = NhsDmdBarcode.find_by(gtin: candidate)
      return nil unless record

      {
        code: record.code,
        display: record.display,
        system: record.system,
        concept_class: record.concept_class,
        source: 'nhs_dmd'
      }
    end
  end
end
