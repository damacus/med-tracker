# frozen_string_literal: true

module Components
  module Icons
    class Calendar < Icons::Base
      def view_template
        svg(**default_attrs) do |s|
          s.rect(width: '18', height: '18', x: '3', y: '4', rx: '2', ry: '2')
          s.line(x1: '16', x2: '16', y1: '2', y2: '6')
          s.line(x1: '8', x2: '8', y1: '2', y2: '6')
          s.line(x1: '3', x2: '21', y1: '10', y2: '10')
        end
      end
    end
  end
end
