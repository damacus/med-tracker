# frozen_string_literal: true

module Components
  module Icons
    class Moon < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M12 3a7 7 0 1 0 9 9 9 9 0 1 1-9-9z')
        end
      end
    end
  end
end
