# frozen_string_literal: true

class MedicationOptionsQuery
  attr_reader :scope

  def initialize(scope:)
    @scope = scope
  end

  def call
    scope.order(:name)
  end

  def include?(medication_id)
    return false if medication_id.blank?

    call.exists?(id: medication_id)
  end
end
