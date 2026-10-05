# frozen_string_literal: true

class MedicationReorderStatusService
  Result = Data.define(:success, :medication) do
    def success?
      success
    end
  end

  def call(medication:, status:, at: Time.current, order_details: {})
    attributes = attributes_for(status, at, order_details)
    return Result.new(success: false, medication: medication) unless attributes

    medication.paper_trail_event = "mark_as_#{status}"
    medication.update!(attributes)
    Result.new(success: true, medication: medication)
  end

  private

  def attributes_for(status, at, order_details)
    case status.to_sym
    when :ordered
      order_attributes(order_details).merge(reorder_status: :ordered, ordered_at: at)
    when :received
      { reorder_status: :received, reordered_at: at }
    end
  end

  def order_attributes(order_details)
    {
      order_supplier: order_details[:supplier].presence,
      order_quantity: order_details[:quantity].presence,
      expected_arrival_on: order_details[:expected_arrival_on].presence
    }
  end
end
