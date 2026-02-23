# frozen_string_literal: true

module Components
  module Icons
    class ChevronRight < ::RubyUI::Base
      def view_template
        svg(
          xmlns: 'http://www.w3.org/2000/svg',
          width: '24',
          height: '24',
          viewbox: '0 0 24 24',
          fill: 'none',
          stroke: 'currentColor',
          stroke_width: '2',
          stroke_linecap: 'round',
          stroke_linejoin: 'round',
          **attrs
        ) do |s|
          s.path(d: 'm9 18 6-6-6-6')
        end
      end
    end
  end
end
