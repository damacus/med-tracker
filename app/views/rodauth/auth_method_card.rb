# frozen_string_literal: true

module Views
  module Rodauth
    class AuthMethodCard < Views::Base
      attr_reader :title, :description, :icon, :enabled, :setup_path, :setup_text, :manage_path, :manage_text

      def initialize(title:, description:, icon:, setup_path:, setup_text:, enabled: false, manage_path: nil, manage_text: nil) # rubocop:disable Metrics/ParameterLists
        @title = title
        @description = description
        @icon = icon
        @enabled = enabled
        @setup_path = setup_path
        @setup_text = setup_text
        @manage_path = manage_path
        @manage_text = manage_text
        super()
      end

      def view_template
        div(class: 'flex items-start gap-4 p-4 rounded-xl border border-slate-200 ' \
                   'bg-white hover:bg-slate-50 transition-colors') do
          render_icon
          render_content
          render_action
        end
      end

      private

      def render_icon
        bg_class = enabled ? 'bg-green-100' : 'bg-slate-100'
        icon_class = enabled ? 'text-green-600' : 'text-slate-500'

        div(class: "flex-shrink-0 w-12 h-12 #{bg_class} rounded-full " \
                   'flex items-center justify-center') do
          send(:"render_#{icon}_icon", icon_class)
        end
      end

      def render_content
        div(class: 'flex-1 min-w-0') do
          div(class: 'flex items-center gap-2') do
            h4(class: 'font-medium text-slate-900') { title }
            render_status_badge if enabled
          end
          p(class: 'text-sm text-slate-600 mt-1') { description }
        end
      end

      def render_action
        div(class: 'flex-shrink-0') do
          if enabled && manage_path
            render RubyUI::Link.new(
              href: manage_path, variant: :outline, size: :sm
            ) { manage_text }
          else
            render RubyUI::Link.new(
              href: setup_path, variant: :primary, size: :sm
            ) { setup_text }
          end
        end
      end

      def render_status_badge
        span(class: 'inline-flex items-center px-2 py-0.5 rounded-full ' \
                    'text-xs font-medium bg-green-100 text-green-800') do
          'Enabled'
        end
      end

      def render_passkey_icon(icon_class)
        svg(
          class: "w-6 h-6 #{icon_class}", fill: 'none',
          viewBox: '0 0 24 24', stroke: 'currentColor', stroke_width: '2'
        ) do |s|
          s.path(
            stroke_linecap: 'round', stroke_linejoin: 'round',
            d: 'M12 11c0 3.517-1.009 6.799-2.753 9.571m-3.44-2.04l.054-.09A13.916 ' \
               '13.916 0 008 11a4 4 0 118 0c0 1.017-.07 2.019-.203 3m-2.118 6.844A21.88 ' \
               '21.88 0 0015.171 17m3.839 1.132c.645-2.266.99-4.659.99-7.132A8 8 0 008 ' \
               '4.07M3 15.364c.64-1.319 1-2.8 1-4.364 0-1.457.39-2.823 1.07-4'
          )
        end
      end

      def render_totp_icon(icon_class)
        svg(
          class: "w-6 h-6 #{icon_class}", fill: 'none',
          viewBox: '0 0 24 24', stroke: 'currentColor', stroke_width: '2'
        ) do |s|
          s.path(
            stroke_linecap: 'round', stroke_linejoin: 'round',
            d: 'M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z'
          )
        end
      end

      def render_recovery_icon(icon_class)
        svg(
          class: "w-6 h-6 #{icon_class}", fill: 'none',
          viewBox: '0 0 24 24', stroke: 'currentColor', stroke_width: '2'
        ) do |s|
          s.path(
            stroke_linecap: 'round', stroke_linejoin: 'round',
            d: 'M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 ' \
               '01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z'
          )
        end
      end
    end
  end
end
