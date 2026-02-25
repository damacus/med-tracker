# frozen_string_literal: true

module RubyUI
  class CalendarNext < Base
    def view_template
      button(**attrs) do
        icon
      end
    end

    private

    def icon
      render ::Components::Icons::ChevronRight.new(size: 16)
    end

    def default_attrs
      {
        name: 'next-month',
        aria_label: 'Go to next month',
        class:
              'inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 border border-input hover:bg-accent hover:text-accent-foreground h-8 w-8 min-h-[32px] min-w-[32px] bg-transparent p-0 opacity-50 hover:opacity-100 absolute right-1',
        type: 'button',
        data_action: 'click->ruby-ui--calendar#nextMonth'
      }
    end
  end
end
