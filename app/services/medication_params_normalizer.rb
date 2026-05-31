# frozen_string_literal: true

class MedicationParamsNormalizer
  BOOLEAN = ActiveModel::Type::Boolean.new

  def self.call(permitted, schedule_config_keys:)
    new(permitted, schedule_config_keys).call
  end

  def initialize(permitted, schedule_config_keys)
    @permitted = permitted
    @schedule_config_keys = schedule_config_keys
  end

  def call
    normalize_default_schedule_config
    normalize_dosage_record_defaults
  end

  private

  attr_reader :permitted, :schedule_config_keys

  def normalize_default_schedule_config
    return unless permitted.key?(:default_schedule_config)

    permitted[:default_schedule_config] = normalized_schedule_config_value(permitted[:default_schedule_config])
  end

  def normalize_dosage_record_defaults
    dosage_records = permitted[:dosage_records_attributes]
    return if dosage_records.blank?

    %i[default_for_adults default_for_children].each do |field|
      normalize_dosage_record_default(dosage_records, field)
    end
  end

  def normalize_dosage_record_default(dosage_records, field)
    selected_records = dosage_records.values
                                     .reject { |attributes| truthy_param?(attributes[:_destroy]) }
                                     .select { |attributes| truthy_param?(attributes[field]) }
    selected_records[0...-1].each { |attributes| attributes[field] = '0' }
  end

  def normalized_schedule_config_value(raw)
    return {} if raw.blank?
    return raw.permit(*schedule_config_keys).to_h if raw.respond_to?(:permit)
    return raw.to_h if raw.respond_to?(:to_h)

    JSON.parse(raw.to_s)
  rescue JSON::ParserError
    {}
  end

  def truthy_param?(value)
    BOOLEAN.cast(value)
  end
end
