# frozen_string_literal: true

module Components
  module Auth
    class BenefitIconTile < Components::Base
      CUSTOM_ICON_COMPONENTS = {
        heart_check: Components::Auth::BenefitIcons::HeartCheck,
        schedule_calendar: Components::Auth::BenefitIcons::ScheduleCalendar,
        progress_path_pin: Components::Auth::BenefitIcons::ProgressPathPin,
        insights_dot_grid_heart: Components::Auth::BenefitIcons::InsightsSearch
      }.freeze

      ICON_COMPONENTS = {
        check_circle: Components::Icons::CheckCircle,
        calendar: Components::Icons::Calendar,
        activity: Components::Icons::Activity
      }.freeze

      def initialize(icon:, color_classes:)
        @icon = icon
        @color_classes = color_classes
        super()
      end

      def view_template
        div(class: "grid h-12 w-12 shrink-0 place-items-center #{frame_classes}") do
          render_icon
        end
      end

      private

      attr_reader :icon, :color_classes

      def frame_classes
        return '' if custom_icon?

        "rounded-lg border #{color_classes}"
      end

      def render_icon
        return render custom_icon_component.new if custom_icon?

        render standard_icon_component.new(size: 24)
      end

      def custom_icon?
        CUSTOM_ICON_COMPONENTS.key?(icon)
      end

      def custom_icon_component
        CUSTOM_ICON_COMPONENTS.fetch(icon)
      end

      def standard_icon_component
        ICON_COMPONENTS.fetch(icon, Components::Icons::Sparkles)
      end
    end
  end
end
