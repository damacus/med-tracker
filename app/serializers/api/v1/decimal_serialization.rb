# frozen_string_literal: true

module Api
  module V1
    module DecimalSerialization
      private

      def decimal_as_json(value)
        return if value.nil?

        BigDecimal(value.to_s).to_s('F')
      end
    end
  end
end
