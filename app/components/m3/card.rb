# frozen_string_literal: true

module Components
  module M3
    class Card < RubyUI::Card
      def initialize(variant: :elevated, **attrs)
        @variant = variant.to_sym
        super(**attrs)
      end

      private

      def default_attrs
        base_classes = 'rounded-shape-xl transition-all'

        variant_classes = case @variant
                          when :elevated
                            'bg-surface-container-low shadow-elevation-1 hover:shadow-elevation-2'
                          when :outlined
                            'bg-surface border border-outline'
                          when :filled
                            'bg-surface-container'
                          else
                            'bg-card border shadow-elevation-1' # Fallback to RubyUI hybrid
                          end

        {
          class: "#{base_classes} #{variant_classes}"
        }
      end
    end

    class CardHeader < RubyUI::CardHeader
      private

      def default_attrs = { class: 'flex flex-col space-y-1.5 p-6' }
    end

    class CardTitle < RubyUI::CardTitle
      private

      def default_attrs = { class: 'text-2xl font-semibold leading-none tracking-tight' }

      def view_template(&)
        render Components::M3::Heading.new(variant: :headline_small, level: 3, **attrs, &)
      end
    end

    class CardDescription < RubyUI::CardDescription
      private

      def default_attrs = { class: 'text-sm text-on-surface-variant' }

      def view_template(&)
        render Components::M3::Text.new(variant: :body_medium, **attrs, &)
      end
    end

    class CardContent < RubyUI::CardContent
      private

      def default_attrs = { class: 'p-6 pt-0' }
    end

    class CardFooter < RubyUI::CardFooter
      private

      def default_attrs = { class: 'flex items-center p-6 pt-0' }
    end
  end
end
