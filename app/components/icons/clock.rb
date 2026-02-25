# frozen_string_literal: true

module Components
  module Icons
    class Clock < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.polyline(points: '12 6 12 12 16 14')
        end
      end
    end
  end
end
