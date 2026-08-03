# frozen_string_literal: true

module Components
  module Layouts
    class DemoNotice < Components::Base
      def view_template
        section(
          role: 'status',
          aria: { labelledby: 'demo-environment-title' },
          class: 'relative z-40 px-4 pt-4 md:ml-64'
        ) do
          Alert(variant: :warning) do
            render Icons::AlertCircle.new(size: 16)
            AlertTitle(id: 'demo-environment-title') { 'Demo environment' }
            AlertDescription do
              plain "This environment contains synthetic and disposable data. #{DemoMode.reset_schedule}."
            end
          end
        end
      end
    end
  end
end
