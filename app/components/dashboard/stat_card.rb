# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a single stat card with title, value, and icon
    class StatCard < Components::Base
      attr_reader :title, :value, :icon_type

      def initialize(title:, value:, icon_type:)
        @title = title
        @value = value
        @icon_type = icon_type
        super()
      end

      def view_template
        Card(class: 'h-full') do
          CardHeader do
            div(class: 'flex items-center justify-between') do
              Heading(level: 2, size: '4', class: 'leading-none tracking-tight font-medium text-slate-600') do
                title
              end
              render_icon
            end
          end
          CardContent do
            Text(size: '8', weight: 'bold', class: 'text-slate-900') { value.to_s }
          end
        end
      end

      private

      def render_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700') do
          case icon_type
          when 'users'
            render Icons::Users.new(size: 20)
          when 'pill'
            render Icons::Pill.new(size: 20)
          end
        end
      end
    end
  end
end
