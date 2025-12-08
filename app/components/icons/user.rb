# frozen_string_literal: true

module Components
  module Icons
    class User < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2')
          s.circle(cx: '12', cy: '7', r: '4')
        end
      end
    end
  end
end
