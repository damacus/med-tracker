# frozen_string_literal: true

ROOT = File.expand_path('../../..', __dir__)
reference_version = File.read(File.join(ROOT, 'rails/Gemfile.lock'))[/^    activesupport \(([^)]+)\)/, 1]
abort 'Cannot identify locked ActiveSupport version' unless reference_version
gem 'activesupport', reference_version
gem 'railties', reference_version
require 'active_support/all'
require 'rails'
require 'active_support/railtie'
require 'digest'
require 'json'
require 'securerandom'

SOURCES = %w[
  rails/app/services/api/record_etag.rb
  rails/app/services/portable_data/export_record_serializer.rb
  rails/app/services/portable_data/export_event_record_serializer.rb
  rails/app/services/portable_data/dose_occurrence_serializer.rb
  rails/app/services/portable_data/exporter.rb
  rails/app/services/portable_data/encryptor.rb
  rails/app/services/households/hosted_export_authorization.rb
  rails/app/services/households/hosted_export.rb
].freeze
SOURCES.each { |path| require File.join(ROOT, path) }
PROVENANCE_SOURCES = (SOURCES + %w[rails/config/application.rb rails/Gemfile.lock]).freeze

class SyntheticRailsConfiguration < Rails::Application; end

defaults = File.read(File.join(ROOT, 'rails/config/application.rb'))[/config\.load_defaults (\d+\.\d+)/, 1]
abort 'Cannot identify retained Rails framework defaults' unless defaults
SyntheticRailsConfiguration.config.load_defaults(defaults)
support_config = SyntheticRailsConfiguration.config.active_support
ActiveSupport::Messages::Codec.default_serializer = support_config.message_serializer
ActiveSupport::Messages::Metadata.use_message_serializer_for_metadata = support_config.use_message_serializer_for_metadata

class SyntheticRecord
  def initialize(**attributes)
    attributes.each { |key, value| define_singleton_method(key) { value } }
  end
end

def record(kind, **attributes)
  SyntheticRecord.new(id: 1, portable_id: "synthetic-rails-#{kind}",
                      updated_at: Time.iso8601('2026-10-12T12:00:00Z'), **attributes)
end

def decimal(value)
  BigDecimal(value)
end

location = record('location', name: 'Synthetic current cupboard', description: 'Fixture storage label')
old_location = record('old-location', name: 'Synthetic former cupboard', description: 'Historical dose location')
preference = record('preference', enabled: true, dose_due_enabled: false, missed_dose_enabled: true,
                    low_stock_enabled: false, private_text_enabled: true,
                    morning_time: Time.iso8601('2000-01-01T08:15:00Z'), afternoon_time: nil,
                    evening_time: Time.iso8601('2000-01-01T19:30:00Z'), night_time: nil)
person = record('person', name: 'Synthetic Dependent', email: 'portable-fixture@example.test',
                date_of_birth: Date.iso8601('1990-01-15'), person_type: 'dependent_adult',
                has_capacity: false, locations: [], notification_preference: preference)
preference.define_singleton_method(:person) { person }
medication = record('medication', location: location, name: 'Synthetic medicine', friendly_name: 'Fixture tablet',
                    category: 'Vitamin', description: 'No real medicine or patient data',
                    dose_amount: decimal('1'), dose_unit: 'tablet', default_schedule_type: 'daily',
                    current_supply: decimal('37.5'), reorder_threshold: decimal('5.5'),
                    barcode: '0000000000000', dmd_code: 'synthetic-dmd', dmd_system: 'synthetic',
                    dmd_concept_class: 'VMP')
dosage = record('dosage', medication: medication, amount: decimal('0.5'), unit: 'tablet', frequency: 'daily',
                description: 'Synthetic half tablet', default_for_adults: true, default_for_children: false,
                default_max_daily_doses: 2, default_min_hours_between_doses: decimal('4'),
                default_dose_cycle: 'daily', current_supply: decimal('8.5'), reorder_threshold: decimal('2.5'))
source_fields = { source_dosage_option: dosage, retired_at: nil, person: person, medication: medication,
                  dose_amount: decimal('0.5'), dose_unit: 'tablet', dose_cycle: 'daily',
                  max_daily_doses: 2, min_hours_between_doses: 4, active: true, notes: 'Synthetic care instructions' }
schedule = record('schedule', **source_fields, frequency: 'daily', schedule_type: 'daily',
                  schedule_config: {}, start_date: Date.iso8601('2026-10-01'), end_date: nil)
assignment = record('assignment', **source_fields, administration_kind: 'routine', position: 1)
take = record('take', client_uuid: '550e8400-e29b-41d4-a716-446655440002',
              schedule: schedule, schedule_id: 1, person_medication: nil,
              taken_at: Time.iso8601('2026-10-10T08:15:00.123456Z'), dose_amount: decimal('0.5'),
              dose_unit: 'tablet', taken_from_medication: medication, taken_from_location: location)
original_actor = SyntheticRecord.new(portable_id: 'synthetic-absent-actor')
original_membership = SyntheticRecord.new(person: original_actor)
pause = record('pause', schedule: schedule, schedule_id: 1, person_medication: nil,
               reason: 'out_of_supply', note: 'Synthetic closed pause',
               started_at: Time.iso8601('2026-10-05T10:00:00.123456Z'),
               ended_at: Time.iso8601('2026-10-06T11:00:00.234567Z'),
               created_at: Time.iso8601('2026-10-05T10:00:00.123456Z'),
               legacy_context: false, imported_context: false,
               recorded_by_membership: original_membership, resumed_by_membership: original_membership,
               imported_actor_references: {})
outcome = record('outcome', schedule_id: 1, source: schedule,
                 window_starts_on: Date.iso8601('2026-10-11'), window_ends_on: Date.iso8601('2026-10-11'),
                 position: 1, scheduled_at: Time.iso8601('2026-10-11T08:15:00Z'),
                 outcome: 'not_taken', reason: 'refused', note: 'Synthetic decision',
                 resolved_at: Time.iso8601('2026-10-11T08:20:00.345678Z'), medication_take: nil)
health = record('health', person: person, event_kind: 'illness', severity: 'mild',
                title: 'Synthetic health event', notes: 'Fixture only',
                started_on: Date.iso8601('2026-10-09'), ended_on: Date.iso8601('2026-10-10'),
                medications: [medication])

class SyntheticExporter < PortableData::Exporter
  def initialize(records)
    @synthetic_records = records
  end

  private

  PortableData::ExportRecordSerializer::COLLECTIONS.each_key do |kind|
    define_method(kind) { |**| @synthetic_records.fetch(kind) }
  end

  def source_instance_id = 'synthetic:rails-serializer'
  def passphrase = 'synthetic portable secret'
  def record_audit_event(*) = nil
end

Time.define_singleton_method(:current) { Time.iso8601('2026-10-12T12:00:00Z') }
synthetic_records = {
  people: [person], locations: [location, old_location], medications: [medication],
  dosage_options: [dosage], schedules: [schedule], person_medications: [assignment],
  medication_takes: [take], notification_preferences: [preference],
  medication_pause_periods: [pause], dose_occurrences: [outcome], health_events: [health]
}
exporter = SyntheticExporter.new(synthetic_records)
identities = synthetic_records.values.flatten.to_h { |item| [item, item.portable_id] }
prepare = lambda do |name, sequence|
  identities.each do |item, identity|
    item.define_singleton_method(:portable_id) { "#{identity}-#{name}" }
  end
  person.define_singleton_method(:email) { "portable-fixture-#{name}@example.test" }
  take.define_singleton_method(:client_uuid) { format('550e8400-e29b-41d4-a716-%012d', sequence) }
  medication.define_singleton_method(:barcode) do
    %w[2000000000015 2000000000022 2000000000039 2000000000046 2000000000053 2000000000060].fetch(sequence - 1)
  end
end
payloads = {}
prepare.call('migration-v1', 1)
payloads['migration-v1'] = exporter.payload
prepare.call('migration-v2', 2)
payloads['migration-v2'] = exporter.v2_payload
prepare.call('household-v2', 3)
payloads['household-v2'] = exporter.household_payload
take.define_singleton_method(:taken_from_location) { old_location }
prepare.call('historical-location-v2', 4)
payloads['historical-location-v2'] = exporter.household_payload
prepare.call('prepared-history-v2', 6)
payloads['prepared-history-v2'] = exporter.household_payload
take.define_singleton_method(:taken_from_location) { location }
person.define_singleton_method(:locations) { [location] }
prepare.call('linked-migration-v2', 5)
payloads['linked-migration-v2'] = exporter.v2_payload
manifest = {
  'source_files_sha256' => PROVENANCE_SOURCES.to_h { |path| [path, Digest::SHA256.file(File.join(ROOT, path)).hexdigest] },
  'ruby_version' => RUBY_VERSION,
  'active_support_version' => ActiveSupport::VERSION::STRING,
  'rails_framework_defaults' => defaults,
  'message_serializer' => support_config.message_serializer.to_s,
  'use_message_serializer_for_metadata' => support_config.use_message_serializer_for_metadata
}
checking = ARGV == ['--check']
abort 'Usage: ruby generate.rb [--check]' unless ARGV.empty? || checking

payloads.each do |name, payload|
  plaintext_path = File.join(__dir__, "#{name}.json")
  envelope_path = File.join(__dir__, "#{name}.encrypted.json")
  serialized = ActiveSupport::JSON.encode(payload)
  if checking
    abort "#{name}: serializer output changed" unless JSON.parse(File.read(plaintext_path)) == JSON.parse(serialized)
    decrypted = PortableData::Encryptor.decrypt(JSON.parse(File.read(envelope_path)), passphrase: 'synthetic portable secret')
    abort "#{name}: encrypted fixture differs" unless decrypted == JSON.parse(serialized)
  else
    File.write(plaintext_path, JSON.pretty_generate(JSON.parse(serialized)) + "\n")
    envelope = File.file?(envelope_path) ? JSON.parse(File.read(envelope_path)) : nil
    unchanged = envelope && PortableData::Encryptor.decrypt(envelope, passphrase: 'synthetic portable secret') == JSON.parse(serialized)
    envelope = PortableData::Encryptor.encrypt(payload, passphrase: 'synthetic portable secret') unless unchanged
    File.write(envelope_path, JSON.pretty_generate(envelope) + "\n")
  end
  next unless payload.fetch(:scope) == 'household'

  export_manifest = Households::HostedExport.send(:export_manifest, SyntheticRecord.new(id: 2702), payload, [])
  archive_entries = Households::HostedExport.send(:archive_entries, payload, export_manifest, {})
  abort "#{name}: hosted payload serialization differs" unless JSON.parse(archive_entries.fetch('portable.json')) == JSON.parse(serialized)
  export_manifest_path = File.join(__dir__, "#{name}.manifest.json")
  if checking
    abort "#{name}: hosted manifest differs" unless JSON.parse(File.read(export_manifest_path)) == JSON.parse(archive_entries.fetch('manifest.json'))
  else
    File.write(export_manifest_path, JSON.pretty_generate(export_manifest) + "\n")
  end
end
manifest_path = File.join(__dir__, 'provenance.json')
if checking
  saved = JSON.parse(File.read(manifest_path))
  abort 'Retained serializer source changed; review and regenerate fixtures' unless saved.fetch('source_files_sha256') == manifest.fetch('source_files_sha256')
else
  File.write(manifest_path, JSON.pretty_generate(manifest) + "\n")
end
puts 'Synthetic Rails serializer fixtures verified'
