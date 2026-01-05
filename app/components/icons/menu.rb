# frozen_string_literal: true

module Components
  module Icons
    class Menu < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.line(x1: '4', x2: '20', y1: '12', y2: '12')
          s.line(x1: '4', x2: '20', y1: '6', y2: '6')
          s.line(x1: '4', x2: '20', y1: '18', y2: '18')
        end
      end
    end
  end
end
