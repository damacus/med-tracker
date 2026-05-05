# frozen_string_literal: true

module MedicationRefillable
  extend ActiveSupport::Concern

  private

  def refill_quantity
    params.dig(:refill, :quantity).to_i
  end

  def parse_restock_date
    Date.parse(params.dig(:refill, :restock_date).to_s)
  rescue ArgumentError
    nil
  end

  def render_refill_error(message)
    respond_to do |format|
      format.html do
        render Components::Medications::ShowView.new(medication: @medication, notice: message), status: :unprocessable_content
      end
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: medication_streams, status: :unprocessable_content
      end
    end
  end

  def medication_streams
    medication = @medication.reload
    [
      turbo_stream.replace("medication_show_#{medication.id}", Components::Medications::ShowView.new(medication: medication)),
      turbo_stream.replace("medication_#{medication.id}", medication_list_item(medication)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def medication_list_item(medication)
    Components::Medications::ListItemComponent.new(
      medication: medication,
      inventory_query_params: {},
      can_manage: policy(medication).update?
    )
  end
end
