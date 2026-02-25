# frozen_string_literal: true

module Components
  module Icons
    class Globe < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20')
          s.path(d: 'M2 12h20')
        end
      end
    end
  end
end
