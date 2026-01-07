# frozen_string_literal: true

module Components
  module Icons
    class LogOut < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4')
          s.polyline(points: '16 17 21 12 16 7')
          s.line(x1: '21', x2: '9', y1: '12', y2: '12')
        end
      end
    end
  end
end
