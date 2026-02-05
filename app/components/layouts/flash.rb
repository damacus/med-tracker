# frozen_string_literal: true

module Components
  module Layouts
    class Flash < Components::Base
      def initialize(notice: nil, alert: nil)
        @notice = notice
        @alert = alert
        super()
      end

      def view_template
        div(class: 'fixed inset-x-0 top-4 z-[60] pointer-events-none') do
          div(class: 'container mx-auto px-4') do
            render_notice if @notice
            render_alert if @alert
          end
        end
      end

      private

      def render_notice
        div(data: { controller: 'flash', flash_dismiss_after_value: 3000 }, class: 'pointer-events-auto') do
          Alert(variant: :success) do
            check_icon
            AlertDescription { @notice }
          end
        end
      end

      def render_alert
        div(data: { controller: 'flash', flash_dismiss_after_value: 0 }, class: 'pointer-events-auto') do
          Alert(variant: :destructive) do
            alert_icon
            AlertDescription { @alert }
          end
        end
      end

      def check_icon
        render Icons::Check.new(size: 16, class: 'lucide lucide-check')
      end

      def alert_icon
        render Icons::AlertCircle.new(size: 16, class: 'lucide lucide-alert-circle')
      end
    end
  end
end
