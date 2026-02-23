# frozen_string_literal: true

module RubyUI
  class BreadcrumbSeparator < Base
    def view_template(&block)
      if block
        li(role: 'presentation', aria_hidden: 'true',
           class: '[&>svg]:w-3.5 [&>svg]:h-3.5', **attrs, &block)
      else
        li(role: 'presentation', aria_hidden: 'true',
           class: '[&>svg]:w-3.5 [&>svg]:h-3.5', **attrs) do
          render ::Components::Icons::ChevronRight.new(class: 'h-4 w-4')
        end
      end
    end
  end
end
