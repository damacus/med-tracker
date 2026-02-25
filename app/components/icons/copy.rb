# frozen_string_literal: true

module Components
  module Icons
    class Copy < Base
      def view_template
        svg(**attrs) do |s|
          s.rect(width: '14', height: '14', x: '8', y: '8', rx: '2', ry: '2')
          s.path(d: 'M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2')
        end
      end
    end
  end
end
