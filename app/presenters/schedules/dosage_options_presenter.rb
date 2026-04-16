# frozen_string_literal: true

module Schedules
  class DosageOptionsPresenter
    attr_reader :schedule, :medications

    def initialize(schedule:, medications:)
      @schedule = schedule
      @medications = medications
    end

    def format_dosage_option(dosage)
      "#{dosage.amount.to_f} #{dosage.unit} - #{dosage.description}"
    end

    def selected_dosage_option
      return @selected_dosage_option if defined?(@selected_dosage_option)

      @selected_dosage_option = dosages.find do |dosage|
        dosage.amount.to_s == schedule.dose_amount.to_s && dosage.unit == schedule.dose_unit
      end
    end

    def selected_dose_selection_key
      selected_dosage_option&.selection_key
    end

    def dosage_dom_id(dosage)
      key = dosage.selection_key.parameterize(separator: '_')
      return "schedule_dose_option_#{key}" unless duplicate_dose_selection_keys.include?(dosage.selection_key)

      description = dosage.description.to_s.parameterize(separator: '_').presence || dosage.object_id
      "schedule_dose_option_#{key}_#{description}"
    end

    def duplicate_dose_selection_keys
      @duplicate_dose_selection_keys ||= dosages.group_by(&:selection_key).each_with_object([]) do |group,
                                                                                                selection_keys|
        selection_key, matching_dosages = group
        selection_keys << selection_key if matching_dosages.size > 1
      end
    end

    def medication_dose_options
      medications.each_with_object({}) do |medication, dose_options|
        dose_options[medication.id.to_s] = medication.dose_options_payload
      end
    end

    private

    def dosages
      schedule.medication&.dosages || []
    end
  end
end
