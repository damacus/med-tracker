# frozen_string_literal: true

module Components
  module Icons
    class Home < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'm3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z')
          s.polyline(points: '9 22 9 12 15 12 15 22')
        end
      end
    end
  end
end
