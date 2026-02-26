# frozen_string_literal: true

module Components
  module Dashboard
    module ScheduleHelpers
      def format_dosage
        amount = schedule.dosage&.amount
        unit = schedule.dosage&.unit
        return '—' unless amount && unit

        formatted_amount = amount == amount.to_i ? amount.to_i : amount
        "#{formatted_amount} #{unit}"
      end

      def format_quantity
        remaining_supply = schedule.medication&.current_supply
        return '—' if remaining_supply.nil?

        remaining_supply.to_s
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
