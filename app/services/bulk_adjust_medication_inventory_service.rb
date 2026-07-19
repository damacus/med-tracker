# frozen_string_literal: true

class BulkAdjustMedicationInventoryService
  Result = Data.define(:success, :medications, :error) do
    def success? = success
  end

  def call(medications:, adjustments:, reason: nil)
    return failure('Select at least one medicine') if adjustments.blank?

    failed_result = nil

    ActiveRecord::Base.transaction do
      medications.sort_by(&:id).each do |medication|
        result = adjust_medication(medication, adjustments, reason)
        next if result.success?

        failed_result = failure(result.error)
        raise ActiveRecord::Rollback
      end
    end

    failed_result || Result.new(success: true, medications: medications, error: nil)
  end

  private

  def adjust_medication(medication, adjustments, reason)
    AdjustMedicationInventoryService.new.call(
      medication: medication,
      new_quantity: adjustments[medication.id.to_s],
      reason: reason
    )
  end

  def failure(error)
    Result.new(success: false, medications: [], error: error)
  end
end
