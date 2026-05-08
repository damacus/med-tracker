# frozen_string_literal: true

module Components
  module Dashboard
    module ScheduleHelpers
      def format_dosage
        amount = schedule.dose_amount
        unit = schedule.dose_unit
        return '—' unless amount && unit

        DoseAmount.new(amount, unit).to_s
      end

      def format_quantity
        remaining_supply = schedule.medication&.current_supply
        return '—' if remaining_supply.nil?

        MedicationStockQuantityFormatter.format(remaining_supply)
      end

      def format_end_date
        schedule.end_date ? schedule.end_date.strftime('%b %d, %Y') : '—'
      end

      def can_delete?
        return false unless current_user

        SchedulePolicy.new(current_user, schedule).destroy?
      end
    end
  end
end
