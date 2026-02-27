# frozen_string_literal: true

module RubyUI
  class BreadcrumbEllipsis < Base
    def view_template(&block)
      if block
        span(role: 'presentation', aria_hidden: 'true',
             class: 'flex h-9 w-9 items-center justify-center', **attrs, &block)
      else
        span(role: 'presentation', aria_hidden: 'true',
             class: 'flex h-9 w-9 items-center justify-center', **attrs) do
          render ::Components::Icons::MoreHorizontal.new(size: 16)
          span(class: 'sr-only') { 'More' }
        end
      end
    end
  end
end
