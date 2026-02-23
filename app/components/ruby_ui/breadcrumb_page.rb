# frozen_string_literal: true

module RubyUI
  class BreadcrumbPage < Base
    def view_template(&)
      span(
        role: 'link',
        aria: { disabled: 'true', current: 'page' },
        class: 'font-normal text-foreground',
        **attrs,
        &
      )
    end
  end
end
