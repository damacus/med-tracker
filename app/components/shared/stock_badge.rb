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
        return if medicine.stock.blank?
        return unless medicine.low_stock? || medicine.out_of_stock?

        render RubyUI::Badge.new(variant: badge_variant) { badge_text }
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
        if medicine.out_of_stock?
          'Out of Stock'
        elsif medicine.low_stock?
          'Low Stock'
        else
          'In Stock'
        end
      end
    end
  end
end
