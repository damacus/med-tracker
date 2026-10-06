# frozen_string_literal: true

module Components
  module Icons
    class FileText < Base
      def view_template
        svg(**attrs) do |s|
          s.path(d: 'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z')
          s.path(d: 'M14 2v4a2 2 0 0 0 2 2h4')
          s.path(d: 'M10 9H8')
          s.path(d: 'M16 13H8')
          s.path(d: 'M16 17H8')
        end
      end
    end
  end
end
