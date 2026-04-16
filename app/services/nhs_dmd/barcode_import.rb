# frozen_string_literal: true

require 'csv'

module NhsDmd
  class BarcodeImport
    Result = Struct.new(:imported_count, :invalid_rows, keyword_init: true)

    REQUIRED_HEADERS = %w[gtin code display].freeze

    def import_csv(path)
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
      record = NhsDmdBarcode.find_or_initialize_by(gtin: attrs[:gtin])
      record.assign_attributes(attrs)
      record.save!
    end

    def build_attrs(row)
      gtin = normalized(row, 'gtin', digits_only: true)
      code = normalized(row, 'code')
      display = normalized(row, 'display')
      return nil if gtin.blank? || code.blank? || display.blank?

      {
        gtin: gtin,
        code: code,
        display: display,
        system: normalized(row, 'system').presence || 'https://dmd.nhs.uk',
        concept_class: normalized(row, 'concept_class').presence
      }
    end

    def normalized(row, key, digits_only: false)
      value = row[key]
      return NhsDmdBarcode.normalize_gtin(value) if digits_only

      value&.strip
    end
  end
end
