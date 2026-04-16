# frozen_string_literal: true

module Components
  module Shared
    class MetricCard < Components::Base
      attr_reader :title, :value, :icon_type, :href, :badge, :testid, :variant, :value_data_attr, :layout

      def initialize(title:, value:, icon_type:, **options)
        @title = title
        @value = value
        @icon_type = icon_type.to_s
        @href = options.fetch(:href, nil)
        @badge = options.fetch(:badge, nil)
        @testid = options.fetch(:testid, nil)
        @variant = options.fetch(:variant, :default).to_sym
        @value_data_attr = options.fetch(:value_data_attr, {}) || {}
        @layout = options.fetch(:layout, :default).to_sym
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
          div(class: wrapper_height_class) { render_card(as_link: false) }
        end
      end

      private

      def render_card(as_link: false)
        m3_card(
          variant: :elevated,
          class: "#{border_class} #{card_height_class} #{min_height_class} " \
                 "shadow-elevation-1 bg-surface-container-low #{hover_class} " \
                 "#{cursor_class} group relative state-layer overflow-hidden",
          data: testid.present? && !as_link ? { testid: testid } : nil
        ) do
          m3_card_content(class: "#{content_padding_class} #{content_height_class} flex flex-col z-10") do
            div(class: "flex items-center justify-between gap-2 #{header_margin_class} min-w-0") do
              m3_text(
                variant: :label_small,
                class: "uppercase font-black tracking-widest truncate #{title_class} text-on-surface-variant"
              ) do
                title
              end
              div(
                class: "#{icon_padding_class} rounded-xl flex-shrink-0 " \
                       "#{icon_bg_class} #{value_color_class} transition-all shadow-inner"
              ) do
                render_icon(size: icon_size)
              end
            end
            div(class: value_wrapper_class) do
              span(
                class: "#{value_size_class} font-black tracking-tight #{value_color_class}",
                data: value_data_attr
              ) do
                value.to_s
              end
              if badge.present?
                m3_badge(
                  variant: :tonal,
                  class: 'px-2.5 py-1 text-[10px] font-black uppercase tracking-wider'
                ) do
                  badge
                end
              end
            end
          end
        end
      end

      def compact?
        layout == :compact
      end

      def wrapper_height_class
        compact? ? '' : 'h-full'
      end

      def card_height_class
        compact? ? '' : 'h-full'
      end

      def content_height_class
        compact? ? '' : 'h-full'
      end

      def value_wrapper_class
        compact? ? 'flex flex-col items-start gap-1' : 'mt-auto flex flex-col items-start gap-2'
      end

      def cursor_class
        href.present? ? 'cursor-pointer' : 'cursor-default'
      end

      def min_height_class
        compact? ? 'min-h-[7rem]' : 'min-h-[9.5rem] sm:min-h-[10rem]'
      end

      def hover_class
        return '' if compact?

        'transition-all duration-300 md:hover:scale-[1.02] md:hover:shadow-elevation-2'
      end

      def content_padding_class
        compact? ? 'p-4' : 'p-6'
      end

      def header_margin_class
        compact? ? 'mb-3' : 'mb-2'
      end

      def title_class
        compact? ? 'text-[0.7rem]' : 'group-hover:text-primary transition-colors'
      end

      def icon_padding_class
        compact? ? 'p-1.5' : 'p-2.5'
      end

      def icon_size
        compact? ? 14 : 20
      end

      def value_size_class
        compact? ? 'text-2xl' : 'text-4xl'
      end

      def background_class
        # No longer used directly, but kept for logic if needed
        variant == :warning ? 'bg-warning-container' : 'bg-surface-container-low'
      end

      def border_class
        variant == :warning ? 'border border-warning/30' : 'border-none'
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
        return 'bg-warning-container/40' if variant == :warning

        case icon_type
        when 'users' then 'bg-primary/10'
        when 'pill' then 'bg-success-container/50'
        else 'bg-secondary-container'
        end
      end

      def value_color_class
        return 'text-on-warning-container' if variant == :warning

        case icon_type
        when 'users' then 'text-primary'
        when 'pill' then 'text-on-success-container'
        when 'check' then 'text-on-secondary-container'
        when 'clock' then 'text-on-warning-container'
        else 'text-foreground'
        end
      end
    end
  end
end
