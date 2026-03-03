# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a single stat card with title, value, and icon
    class StatCard < Components::Base
      attr_reader :title, :value, :icon_type, :href

      def initialize(title:, value:, icon_type:, href: nil)
        @title = title
        @value = value
        @icon_type = icon_type
        @href = href
        super()
      end

      def view_template
        render_card
      end

      private

      def render_card
        render Components::Shared::MetricCard.new(
          title: title,
          value: value,
          icon_type: icon_type,
          href: href
        )
      end
    end
  end
end
