# frozen_string_literal: true

module RubyUI
  class InlineCode < Base
    def view_template(&)
      code(**attrs, &)
    end

    private

    def default_attrs
      {
        class: 'relative rounded bg-secondary-container px-[0.3rem] py-[0.2rem] font-mono text-sm font-semibold'
      }
    end
  end
end
