# frozen_string_literal: true

module RubyUI
  class BreadcrumbLink < Base
    def view_template(&)
      a(class: 'transition-colors hover:text-foreground', **attrs, &)
    end
  end
end
