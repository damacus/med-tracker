# frozen_string_literal: true

module RubyUI
  class BreadcrumbItem < Base
    def view_template(&)
      li(class: 'inline-flex items-center gap-1.5', **attrs, &)
    end
  end
end
