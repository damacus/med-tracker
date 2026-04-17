# frozen_string_literal: true

module RubyUI
  class DivWrapper < Base
    DEFAULT_CLASS = ''

    def view_template(&)
      div(**attrs, &)
    end

    private

    def default_attrs
      {
        class: self.class::DEFAULT_CLASS
      }
    end
  end
end
