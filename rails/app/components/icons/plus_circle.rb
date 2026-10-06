# frozen_string_literal: true

module Components
  module Icons
    class PlusCircle < Base
      def view_template
        svg(**attrs) do |s|
          s.circle(cx: '12', cy: '12', r: '10')
          s.path(d: 'M8 12h8')
          s.path(d: 'M12 8v8')
        end
      end
    end
  end
end
