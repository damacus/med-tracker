# frozen_string_literal: true

module Components
  module Icons
    class Send < Base
      def view_template
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: size,
          height: size,
          viewbox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round',
          class: @class
        ) do |s|
          s.line(x1: '22', y1: '2', x2: '11', y2: '13')
          s.polygon(points: '22 2 15 22 11 13 2 9 22 2')
        end
      end
    end
  end
end
