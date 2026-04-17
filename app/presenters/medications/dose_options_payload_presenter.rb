# frozen_string_literal: true

module Medications
  class DoseOptionsPayloadPresenter
    attr_reader :medications

    def initialize(medications:)
      @medications = medications
    end

    def to_h
      medications.each_with_object({}) do |medication, dose_options|
        dose_options[medication.id.to_s] = medication.dose_options_payload
      end
    end
  end
end
