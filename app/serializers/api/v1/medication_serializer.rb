# frozen_string_literal: true

module Api
  module V1
    class MedicationSerializer
      def initialize(medication)
        @medication = medication
      end

      def as_json(*)
        details_data.merge(stock_data)
      end

      private

      attr_reader :medication

      def details_data
        identity_data.merge(inventory_data)
      end

      def identity_data
        {
          id: medication.id,
          name: medication.name,
          category: medication.category,
          description: medication.description,
          dosage_amount: medication.dosage_amount,
          dosage_unit: medication.dosage_unit
        }
      end

      def inventory_data
        {
          current_supply: medication.current_supply,
          reorder_threshold: medication.reorder_threshold,
          reorder_status: medication.reorder_status,
          location_id: medication.location_id,
          updated_at: medication.updated_at.iso8601
        }
      end

      def stock_data
        {
          low_stock: medication.low_stock?,
          out_of_stock: medication.out_of_stock?,
          days_until_low_stock: medication.days_until_low_stock,
          days_until_out_of_stock: medication.days_until_out_of_stock
        }
      end
    end
  end
end
