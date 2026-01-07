# frozen_string_literal: true

module Components
  module Icons
    class Search < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.circle(cx: '11', cy: '11', r: '8')
          s.path(d: 'm21 21-4.3-4.3')
        end
      end
    end
  end
end
