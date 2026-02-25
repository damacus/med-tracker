# frozen_string_literal: true

module Components
  module Icons
    class Plus < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(
            stroke_linecap: 'round',
            stroke_linejoin: 'round',
            d: 'M12 4.5v15m7.5-7.5h-15'
          )
        end
      end
    end
  end
end
