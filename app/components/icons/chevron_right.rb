# frozen_string_literal: true

module Components
  module Icons
    class ChevronRight < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M9 5L16 12L9 19')
        end
      end

      private

      def default_attrs
        super.merge(stroke_width: '2.5')
      end
    end
  end
end
