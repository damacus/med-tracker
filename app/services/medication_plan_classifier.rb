# frozen_string_literal: true

class MedicationPlanClassifier
  SUPPLEMENT_CATEGORIES = %w[vitamin supplement mineral].freeze

  attr_reader :medication

  def initialize(medication:, schedule_type: nil)
    @medication = medication
    @schedule_type = schedule_type
  end

  def direct?
    supplement_category? || schedule_type == 'prn'
  end

  def administration_kind
    supplement_category? ? 'routine' : 'as_needed'
  end

  def schedule_type
    @schedule_type.presence || medication.default_schedule_type.presence || 'multiple_daily'
  end

  private

  def supplement_category?
    SUPPLEMENT_CATEGORIES.include?(medication.category.to_s.downcase)
  end
end
