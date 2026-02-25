# frozen_string_literal: true

module Components
  module Icons
    class Lock < Base
      def view_template
        svg(**attrs) do |s|
          s.rect(width: '18', height: '11', x: '3', y: '11', rx: '2', ry: '2')
          s.path(d: 'M7 11V7a5 5 0 0 1 10 0v4')
        end
      end
    end
  end
end
