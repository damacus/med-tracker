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
        div(class: 'container mx-auto px-4 py-4') do
          render_notice if @notice
          render_alert if @alert
        end
      end

      private

      def render_notice
        Alert(variant: :success) do
          check_icon
          AlertTitle { 'Success' }
          AlertDescription { @notice }
        end
      end

      def render_alert
        Alert(variant: :destructive) do
          alert_icon
          AlertTitle { 'Error' }
          AlertDescription { @alert }
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
