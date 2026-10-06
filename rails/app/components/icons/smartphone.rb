# frozen_string_literal: true

module Components
  module Icons
    class Smartphone < Base
      def view_template
        svg(**attrs) do |s|
          s.rect(width: '14', height: '20', x: '5', y: '2', rx: '2', ry: '2')
          s.path(d: 'M12 18h.01')
        end
      end
    end
  end
end
