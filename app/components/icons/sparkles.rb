# frozen_string_literal: true

module Components
  module Icons
    class Sparkles < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M12 3l1.8 4.2L18 9l-4.2 1.8L12 15l-1.8-4.2L6 9l4.2-1.8L12 3z')
          s.path(d: 'M5 16l.9 2.1L8 19l-2.1.9L5 22l-.9-2.1L2 19l2.1-.9L5 16z')
          s.path(d: 'M19 13l.9 2.1L22 16l-2.1.9L19 19l-.9-2.1L16 16l2.1-.9L19 13z')
        end
      end
    end
  end
end
