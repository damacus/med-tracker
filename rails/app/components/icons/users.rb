# frozen_string_literal: true

module Components
  module Icons
    class Users < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2')
          s.circle(cx: '9', cy: '7', r: '4')
          s.path(d: 'M22 21v-2a4 4 0 0 0-3-3.87')
          s.path(d: 'M16 3.13a4 4 0 0 1 0 7.75')
        end
      end
    end
  end
end
