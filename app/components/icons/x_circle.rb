# frozen_string_literal: true

module Components
  module Icons
    class XCircle < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'm15 9-6 6')
          s.path(d: 'm9 9 6 6')
        end
      end
    end
  end
end
