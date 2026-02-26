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
        if medication.out_of_stock?
          :destructive
        elsif medication.low_stock?
          :warning
        else
          :outline
        end
      end

      def badge_text
        count = medication.current_supply.to_i
        if medication.out_of_stock?
          "Out of Stock (#{count})"
        elsif medication.low_stock?
          "Low Stock (#{count})"
        else
          "#{count} left"
        end
      end
    end
  end
end
