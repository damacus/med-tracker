# frozen_string_literal: true

module Components
  module Shared
    class MetricCard < Components::Base
      attr_reader :title, :value, :icon_type, :href, :badge, :testid, :variant, :value_data_attr

      def initialize(title:, value:, icon_type:, **options)
        @title = title
        @value = value
        @icon_type = icon_type.to_s
        @href = options.fetch(:href, nil)
        @badge = options.fetch(:badge, nil)
        @testid = options.fetch(:testid, nil)
        @variant = options.fetch(:variant, :default).to_sym
        @value_data_attr = options.fetch(:value_data_attr, {}) || {}
        super()
      end

      def view_template
        if href.present?
          a(
            href: href,
            data: testid.present? ? { testid: testid } : nil,
            class: 'block h-full no-underline focus-visible:outline-none ' \
                   'focus-visible:ring-2 focus-visible:ring-primary ' \
                   'focus-visible:ring-offset-2 rounded-[2rem]'
          ) do
            render_card(as_link: true)
          end
        else
          div(class: 'h-full') { render_card(as_link: false) }
        end
      end

      private

      def render_card(as_link: false)
        Card(
          class: "#{border_class} h-full min-h-[9.5rem] sm:min-h-[10rem] " \
                 "shadow-sm #{background_class} backdrop-blur-sm " \
                 'transition-all duration-300 md:hover:scale-[1.02] md:hover:shadow-xl md:hover:shadow-primary/5 ' \
                 "#{cursor_class} group",
          data: testid.present? && !as_link ? { testid: testid } : nil
        ) do
          CardContent(class: 'p-6 h-full flex flex-col') do
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
            div(class: 'mt-auto flex flex-col items-start gap-2') do
              span(class: "text-3xl font-black tracking-tight #{value_color_class}", data: value_data_attr) do
                value.to_s
              end
              if badge.present?
                span(
                  class: 'inline-flex items-center rounded-full px-2.5 py-1 text-xs font-semibold ' \
                         'bg-slate-100 text-slate-700'
                ) do
                  badge
                end
              end
            end
          end
        end
      end

      def cursor_class
        href.present? ? 'cursor-pointer' : 'cursor-default'
      end

      def background_class
        variant == :warning ? 'bg-amber-50/50' : 'bg-white/50'
      end

      def border_class
        variant == :warning ? 'border-amber-200' : 'border-none'
      end

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
        return 'bg-amber-100' if variant == :warning

        case icon_type
        when 'users' then 'bg-blue-50'
        when 'pill' then 'bg-emerald-50'
        when 'check' then 'bg-indigo-50'
        when 'clock' then 'bg-amber-50'
        else 'bg-slate-50'
        end
      end

      def value_color_class
        return 'text-amber-700' if variant == :warning

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
