# frozen_string_literal: true

module Components
  module Shared
    # Renders a stock status badge for a medicine
    # Shows "In Stock", "Low Stock", or "Out of Stock" with quantity
    class StockBadge < Components::Base
      attr_reader :medicine

      def initialize(medicine:)
        @medicine = medicine
        super()
      end

      def view_template
        return if medicine.current_supply.blank?

        render RubyUI::Badge.new(variant: badge_variant, class: 'rounded-full text-[10px] py-0.5 px-2') { badge_text }
      end

      private

      def badge_variant
        if medicine.out_of_stock?
          :destructive
        elsif medicine.low_stock?
          :warning
        else
          :outline
        end
      end

      def badge_text
        count = medicine.current_supply.to_i
        if medicine.out_of_stock?
          "Out of Stock (#{count})"
        elsif medicine.low_stock?
          "Low Stock (#{count})"
        else
          "#{count} left"
        end
      end
    end
  end
end
