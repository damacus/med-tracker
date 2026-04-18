# frozen_string_literal: true

dir = ARGV.first.to_s

raise ArgumentError, 'Usage: rails runner db/seeds/import_nhs_dmd_release.rb /path/to/release/dir' if dir.blank?

result = NhsDmd::ReleaseImport.new.import(dir)
puts "Imported: #{result.imported_count}, Skipped: #{result.skipped_count}"
