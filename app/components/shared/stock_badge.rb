# frozen_string_literal: true

module Components
  module Shared
    # Renders a stock status badge for a medication
    # Shows "In Stock", "Low Stock", or "Out of Stock" with quantity
    class StockBadge < Components::Base
      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        return if medication.current_supply.blank?

        render RubyUI::Badge.new(variant: badge_variant, class: 'rounded-full text-[10px] py-0.5 px-2') { badge_text }
      end

      private

      def badge_variant
        case stock_status
        when :out_of_stock then :destructive
        when :low_stock then :warning
        else :outline
        end
      end

      def badge_text
        count = medication.current_supply.to_i

        case stock_status
        when :out_of_stock then "Out of Stock (#{count})"
        when :low_stock then "Low Stock (#{count})"
        else "#{count} left"
        end
      end

      def stock_status
        medication.supply_level.status
      end
    end
  end
end
