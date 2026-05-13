# frozen_string_literal: true

module Components
  module Icons
    class Droplet < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M12 22a7 7 0 0 0 7-7c0-2-1-3.9-3-5.5L12 2 8.1 9.5C6.1 11.1 5 13 5 15a7 7 0 0 0 7 7z')
        end
      end
    end
  end
end
