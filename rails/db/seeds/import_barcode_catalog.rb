# frozen_string_literal: true

path = ARGV.first.to_s

raise ArgumentError, 'Usage: rails runner db/seeds/import_barcode_catalog.rb path/to/file.csv' if path.blank?

result = BarcodeCatalog::ImportCsv.new.call(path)
puts({ imported_count: result.imported_count, invalid_rows: result.invalid_rows }.inspect)
