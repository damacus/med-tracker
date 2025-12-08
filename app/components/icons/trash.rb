# frozen_string_literal: true

module Components
  module Icons
    class Trash < Base
      def view_template
        svg(**merged_attrs) do |s|
          s.path(d: 'M3 6h18')
          s.path(d: 'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6')
          s.path(d: 'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2')
          s.line(x1: '10', y1: '11', x2: '10', y2: '17')
          s.line(x1: '14', y1: '11', x2: '14', y2: '17')
        end
      end
    end
  end
end
