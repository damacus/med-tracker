# frozen_string_literal: true

require 'csv'

module BarcodeCatalog
  class ImportCsv
    Result = Struct.new(:imported_count, :invalid_rows, keyword_init: true)

    REQUIRED_HEADERS = %w[gtin display source].freeze

    def call(path)
      invalid_rows = []
      imported_count = 0

      csv_rows(path).each_with_index do |row, index|
        attrs = build_attrs(row)
        if attrs
          persist(attrs)
          imported_count += 1
        else
          invalid_rows << (index + 2)
        end
      end

      Result.new(imported_count: imported_count, invalid_rows: invalid_rows)
    end

    private

    def csv_rows(path)
      rows = CSV.read(path, headers: true)
      validate_headers!(rows.headers)
      rows
    end

    def validate_headers!(headers)
      normalized_headers = headers.compact.map(&:strip)
      missing_headers = REQUIRED_HEADERS - normalized_headers
      raise ArgumentError, "Missing required headers: #{missing_headers.join(', ')}" if missing_headers.any?
    end

    def persist(attrs)
      record = BarcodeCatalogEntry.find_or_initialize_by(gtin: attrs[:gtin], source: attrs[:source])
      record.assign_attributes(attrs)
      record.save!
    end

    def build_attrs(row)
      gtin = normalized(row, 'gtin', digits_only: true)
      display = normalized(row, 'display')
      source = normalized(row, 'source')
      return nil if gtin.blank? || display.blank? || source.blank?

      {
        gtin: gtin,
        display: display,
        source: source,
        code: normalized(row, 'code').presence,
        system: normalized(row, 'system').presence,
        concept_class: normalized(row, 'concept_class').presence
      }
    end

    def normalized(row, key, digits_only: false)
      value = row[key]
      return BarcodeCatalogEntry.normalize_gtin(value) if digits_only

      value&.strip
    end
  end
end
