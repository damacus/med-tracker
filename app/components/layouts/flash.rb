# frozen_string_literal: true

module Components
  module Layouts
    class Flash < Components::Base
      def initialize(notice: nil, alert: nil, warning: nil)
        @notice = notice
        @alert = alert
        @warning = warning
        super()
      end

      def view_template
        div(class: content_class) do
          render_notice if @notice
          render_warning if @warning
          render_alert if @alert
        end
      end

      private

      def content_class
        'container mx-auto space-y-2 px-4'
      end

      def render_notice
        div(data: { controller: 'flash', flash_dismiss_after_value: 3000 }, class: 'pointer-events-auto') do
          Alert(variant: :success) do
            check_icon
            AlertDescription { @notice }
          end
        end
      end

      def render_warning
        div(data: { controller: 'flash', flash_dismiss_after_value: 8000 }, class: 'pointer-events-auto') do
          Alert(variant: :warning, class: 'relative pr-12') do
            alert_circle_icon
            AlertDescription { @warning }
            dismiss_button
          end
        end
      end

      def render_alert
        div(data: { controller: 'flash', flash_dismiss_after_value: 0 }, class: 'pointer-events-auto') do
          Alert(variant: :destructive) do
            alert_circle_icon
            AlertDescription { @alert }
          end
        end
      end

      def check_icon
        render Icons::Check.new(size: 16)
      end

      def alert_circle_icon
        render Icons::AlertCircle.new(size: 16)
      end

      def dismiss_button
        button(
          type: 'button',
          class: 'absolute right-2 top-4 flex h-9 w-9 items-center justify-center rounded-full p-0 ' \
                 'text-current opacity-70 hover:bg-black/5 hover:opacity-100 focus-visible:outline-none ' \
                 'focus-visible:ring-2 focus-visible:ring-current',
          aria: { label: I18n.t('ruby_ui.common.close') },
          data: { action: 'click->flash#dismiss' }
        ) do
          render Icons::X.new(
            size: 16,
            class: 'absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2',
            aria_hidden: 'true'
          )
        end
      end
    end
  end
end
