# frozen_string_literal: true

class MedicationReorderStatusService
  Result = Data.define(:success, :medication) do
    def success?
      success
    end
  end

  def call(medication:, status:, at: Time.current)
    attributes = attributes_for(status, at)
    return Result.new(success: false, medication: medication) unless attributes

    medication.paper_trail_event = "mark_as_#{status}"
    medication.update!(attributes)
    Result.new(success: true, medication: medication)
  end

  private

  def attributes_for(status, at)
    case status.to_sym
    when :ordered
      { reorder_status: :ordered, ordered_at: at }
    when :received
      { reorder_status: :received, reordered_at: at }
    end
  end
end
