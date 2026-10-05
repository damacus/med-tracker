# frozen_string_literal: true

module Components
  module Icons
    class Plus < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M5 12h14')
          s.path(d: 'M12 5v14')
        end
      end
    end
  end
end
