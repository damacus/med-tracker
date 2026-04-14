# frozen_string_literal: true

module Views
  module Profiles
    class DangerZoneCard < Components::Base
      def view_template
        render Card.new(class: 'border-destructive/35 bg-card shadow-elevation-2') do
          render CardHeader.new do
            render CardTitle.new(class: 'text-destructive') { 'Danger Zone' }
          end
          render CardContent.new do
            render CloseAccountDialog.new
          end
        end
      end
    end
  end
end
