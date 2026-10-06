# frozen_string_literal: true

module Components
  module Icons
    class AlertCircle < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'M12 8v4')
          s.path(d: 'M12 16h.01')
        end
      end
    end
  end
end
