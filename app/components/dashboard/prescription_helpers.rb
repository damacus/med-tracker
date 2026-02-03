# frozen_string_literal: true

module Components
  module Dashboard
    module PrescriptionHelpers
      def take_now_classes
        'inline-flex items-center justify-center rounded-full text-sm font-medium transition-colors ' \
          'min-h-[44px] min-w-[44px] px-4 py-2 bg-green-100 text-green-700 hover:bg-green-200'
      end

      def delete_classes
        'inline-flex items-center justify-center rounded-full text-sm font-medium transition-colors ' \
          'min-h-[44px] min-w-[44px] px-4 py-2 bg-red-100 text-red-700 hover:bg-red-200'
      end

      def format_dosage
        amount = prescription.dosage&.amount
        unit = prescription.dosage&.unit
        return '—' unless amount && unit

        formatted_amount = amount == amount.to_i ? amount.to_i : amount
        "#{formatted_amount}#{unit}"
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
