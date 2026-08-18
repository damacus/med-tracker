# frozen_string_literal: true

module Components
  module Layouts
    class DemoNotice < Components::Base
      def view_template
        section(
          role: 'status',
          aria: { labelledby: 'demo-environment-title' },
          class: 'pointer-events-auto px-4',
          data: { controller: 'flash', flash_dismiss_after_value: 0 }
        ) do
          Alert(variant: :warning, class: 'relative pr-12') do
            render Icons::AlertCircle.new(size: 16)
            AlertTitle(id: 'demo-environment-title') { 'Demo environment' }
            AlertDescription do
              plain "This environment contains synthetic and disposable data. #{DemoMode.reset_schedule}."
            end
            dismiss_button
          end
        end
      end

      private

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
