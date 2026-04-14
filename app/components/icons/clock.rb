# frozen_string_literal: true

module Components
  module Icons
    class Clock < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'M12 6v6l4 2')
        end
      end
    end
  end
end
