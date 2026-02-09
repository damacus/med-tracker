# frozen_string_literal: true

module Components
  module Dashboard
    module PrescriptionHelpers
      def format_dosage
        amount = prescription.dosage&.amount
        unit = prescription.dosage&.unit
        return '—' unless amount && unit

        formatted_amount = amount == amount.to_i ? amount.to_i : amount
        "#{formatted_amount} #{unit}"
      end

      def format_end_date
        prescription.end_date ? prescription.end_date.strftime('%b %d, %Y') : '—'
      end

      def can_delete?
        return false unless current_user

        PrescriptionPolicy.new(current_user, prescription).destroy?
      end
    end
  end
end
