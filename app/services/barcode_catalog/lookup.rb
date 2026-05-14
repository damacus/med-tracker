# frozen_string_literal: true

module BarcodeCatalog
  class Lookup
    def initialize(opf_lookup: OpenProductsFacts::BarcodeLookup.new)
      @opf_lookup = opf_lookup
    end

    def lookup(barcode)
      barcode_candidates(barcode).each do |candidate|
        imported = lookup_imported_catalog(candidate)
        return imported if imported

        local = lookup_local(candidate)
        return local if local

        cached_opf = lookup_cached_open_products_facts(candidate)
        return cached_opf if cached_opf

        opf = lookup_open_products_facts(candidate)
        return opf if opf

        curated = lookup_curated(candidate)
        return curated if curated
      end

      nil
    end

    private

    def barcode_candidates(barcode)
      NhsDmd::BarcodeLookup.candidates_for(barcode)
    end

    def lookup_imported_catalog(candidate)
      record = BarcodeCatalogEntry.where(gtin: candidate).where.not(source: 'open_products_facts').first
      catalog_entry_attributes(record)
    end

    def lookup_cached_open_products_facts(candidate)
      record = BarcodeCatalogEntry.find_by(gtin: candidate, source: 'open_products_facts')
      catalog_entry_attributes(record)
    end

    def catalog_entry_attributes(record)
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
        display: record.vmp_name.presence || record.display,
        system: record.system,
        concept_class: record.concept_class,
        source: 'nhs_dmd'
      }
    end

    def lookup_open_products_facts(candidate)
      @opf_lookup.lookup(candidate)
    end

    def lookup_curated(candidate)
      BarcodeCatalog::CuratedProducts.lookup_gtin(candidate)&.lookup_attributes
    end
  end
end
