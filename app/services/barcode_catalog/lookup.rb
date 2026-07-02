# frozen_string_literal: true

module BarcodeCatalog
  class Lookup
    SOURCE_LOOKUPS = {
      'imported_catalog' => :lookup_imported_catalog,
      'local_nhs_dmd' => :lookup_local,
      'cached_open_products_facts' => :lookup_cached_open_products_facts,
      'open_products_facts' => :lookup_open_products_facts,
      'curated_catalog' => :lookup_curated
    }.freeze

    def initialize(opf_lookup: OpenProductsFacts::BarcodeLookup.new)
      @opf_lookup = opf_lookup
    end

    def lookup(barcode)
      barcode_candidates(barcode).each do |candidate|
        configured_source_lookups.each do |lookup_method|
          result = send(lookup_method, candidate)
          return result if result
        end
      end

      nil
    end

    private

    def barcode_candidates(barcode)
      NhsDmd::BarcodeLookup.candidates_for(barcode)
    end

    def configured_source_lookups
      AppSettings.instance.lookup_source_priority_for(SOURCE_LOOKUPS.keys).map { |source| SOURCE_LOOKUPS.fetch(source) }
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
