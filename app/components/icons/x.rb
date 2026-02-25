# frozen_string_literal: true

module Components
  module Icons
    class X < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(
            stroke_linecap: 'round',
            stroke_linejoin: 'round',
            d: 'M6 18L18 6M6 6l12 12'
          )
        end
      end
    end
  end
end
