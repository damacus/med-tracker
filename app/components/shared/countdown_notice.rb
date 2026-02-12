# frozen_string_literal: true

module Components
  module Shared
    class CountdownNotice < Components::Base
      attr_reader :countdown_display

      def initialize(countdown_display:)
        @countdown_display = countdown_display
        super()
      end

      def view_template
        return if countdown_display.blank?

        div(class: 'p-3 bg-amber-50 border border-amber-200 rounded-md') do
          Text(size: '2', class: 'text-amber-800') do
            span(class: 'font-semibold') { '🕐 Next dose available in: ' }
            plain countdown_display
          end
        end
      end
    end
  end
end
