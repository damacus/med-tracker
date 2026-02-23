# frozen_string_literal: true

module RubyUI
  class Breadcrumbs < Base
    def view_template(&)
      nav(aria: { label: 'breadcrumb' }, **attrs) do
        ol(class: 'flex flex-wrap items-center gap-1.5 break-words text-sm text-muted-foreground sm:gap-2.5',
           &)
      end
    end
  end
end
