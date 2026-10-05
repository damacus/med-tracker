# frozen_string_literal: true

module Components
  module Icons
    class MoreHorizontal < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '1')
          s.circle(cx: '19', cy: '12', r: '1')
          s.circle(cx: '5', cy: '12', r: '1')
        end
      end
    end
  end
end
