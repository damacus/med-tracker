# frozen_string_literal: true

module Components
  module Icons
    class LogOut < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4')
          s.path(d: 'm16 17 5-5-5-5')
          s.line(x1: '21', x2: '9', y1: '12', y2: '12')
        end
      end
    end
  end
end
