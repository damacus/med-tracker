# frozen_string_literal: true

module Components
  module M3
    class Select < RubyUI::Base
      FORM_FIELD_ACTIONS = [
        'change->ruby-ui--form-field#onChange',
        'invalid->ruby-ui--form-field#onInvalid'
      ].freeze

      SIZE_CLASSES = {
        md: 'h-14 min-h-[56px] px-4 py-4 text-base',
        sm: 'h-9 px-3 py-2 text-sm shadow-sm'
      }.freeze

      def initialize(size: :md, **attrs)
        @size = size
        data = attrs.delete(:data) || {}
        super(**attrs, data: merge_form_field_data(data))
      end

      def view_template(&)
        select(**attrs, &)
      end

      private

      attr_reader :size

      def default_attrs
        {
          class: [
            'flex w-full rounded-shape-sm border border-outline bg-transparent text-foreground transition-all',
            'ring-offset-background disabled:cursor-not-allowed disabled:opacity-38',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary',
            size_classes
          ].join(' ')
        }
      end

      def size_classes
        SIZE_CLASSES.fetch(size.to_sym)
      end

      def merge_form_field_data(data)
        data = data.to_h.dup
        user_action = data.delete(:action) || data.delete('action')

        data.merge(
          ruby_ui__form_field_target: 'input',
          action: [*FORM_FIELD_ACTIONS, *Array(user_action)].compact.join(' ')
        )
      end
    end
  end
end
