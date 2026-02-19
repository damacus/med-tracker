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
        Card(
          class: 'border-none shadow-sm bg-white/50 backdrop-blur-sm transition-all duration-300 ' \
                 'hover:scale-[1.03] hover:shadow-xl hover:shadow-primary/5 cursor-default group'
        ) do
          CardContent(class: 'p-6 relative overflow-hidden') do
            # Subtle background icon for depth
            div(
              class: 'absolute -right-3 -bottom-3 opacity-5 group-hover:scale-110 transition-transform ' \
                     'duration-500 text-slate-900'
            ) do
              render_icon(size: 48)
            end

            div(class: 'relative z-10 space-y-1') do
              div(class: 'flex items-center justify-between gap-2 mb-2 min-w-0') do
                Text(
                  size: '1', weight: 'muted',
                  class: 'uppercase font-black tracking-widest group-hover:text-primary transition-colors truncate'
                ) do
                  title
                end
                div(class: "p-2 rounded-lg flex-shrink-0 #{icon_bg_class} #{value_color_class} transition-colors") do
                  render_icon(size: 16)
                end
              end
              div(class: 'flex items-baseline gap-2') do
                span(class: "text-3xl font-black tracking-tight #{value_color_class}") { value.to_s }
              end
            end
          end
        end
      end

      private

      def render_icon(size:)
        case icon_type
        when 'users' then render Icons::Users.new(size: size)
        when 'pill' then render Icons::Pill.new(size: size)
        when 'check' then render Icons::CheckCircle.new(size: size)
        when 'clock' then render Icons::Clock.new(size: size)
        else render Icons::Activity.new(size: size)
        end
      end

      def icon_bg_class
        case icon_type
        when 'users' then 'bg-blue-50'
        when 'pill' then 'bg-emerald-50'
        when 'check' then 'bg-indigo-50'
        when 'clock' then 'bg-amber-50'
        else 'bg-slate-50'
        end
      end

      def value_color_class
        case icon_type
        when 'users' then 'text-blue-600'
        when 'pill' then 'text-emerald-600'
        when 'check' then 'text-indigo-600'
        when 'clock' then 'text-amber-600'
        else 'text-slate-900'
        end
      end
    end
  end
end
