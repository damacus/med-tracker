# frozen_string_literal: true

module Schedules
  class DosageOptionsPresenter
    attr_reader :schedule

    def initialize(schedule:)
      @schedule = schedule
    end

    def format_dosage_option(dosage)
      "#{dosage.amount.to_f} #{dosage.unit} - #{dosage.description}"
    end

    def selected_dosage_option
      return @selected_dosage_option if defined?(@selected_dosage_option)

      @selected_dosage_option = dosage_selected_by_id || dosage_selected_by_snapshot
    end

    def selected_dose_selection_key
      selected_dosage_option&.selection_key
    end

    def selected_dose_option_value
      selected_dosage_option&.option_value
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

    def dosages
      return @dosages if defined?(@dosages)

      @dosages = schedule.medication ? schedule.medication.dosage_records.order(:amount, :id).to_a : []
    end

    private

    def dosage_selected_by_id
      return if schedule.source_dosage_option_id.blank?

      dosages.find { |dosage| dosage.id == schedule.source_dosage_option_id }
    end

    def dosage_selected_by_snapshot
      dosages.find { |dosage| dosage.amount.to_s == schedule.dose_amount.to_s && dosage.unit == schedule.dose_unit }
    end
  end
end
