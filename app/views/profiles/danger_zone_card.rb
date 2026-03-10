# frozen_string_literal: true

module Views
  module Profiles
    class DangerZoneCard < Components::Base
      def view_template
        render Card.new(class: 'rounded-[2rem] border-destructive/70 bg-[linear-gradient(135deg,rgba(255,255,255,0.98),rgba(255,240,240,0.92))] shadow-[0_18px_45px_-32px_rgba(127,29,29,0.35)] dark:bg-[linear-gradient(135deg,rgba(44,18,22,0.92),rgba(60,20,28,0.88))]') do
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
