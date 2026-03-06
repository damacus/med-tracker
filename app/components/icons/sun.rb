# frozen_string_literal: true

module Components
  module Icons
    class Sun < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '4')
          s.path(d: 'M12 2v2')
          s.path(d: 'M12 20v2')
          s.path(d: 'm4.93 4.93 1.41 1.41')
          s.path(d: 'm17.66 17.66 1.41 1.41')
          s.path(d: 'M2 12h2')
          s.path(d: 'M20 12h2')
          s.path(d: 'm6.34 17.66-1.41 1.41')
          s.path(d: 'm19.07 4.93-1.41 1.41')
        end
      end
    end
  end
end
