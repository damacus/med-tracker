# frozen_string_literal: true

module Components
  module M3
    class SelectableOption < RubyUI::Base
      def initialize(**attrs)
        component_attrs = attrs.dup
        @type = component_attrs.delete(:type)
        @name = component_attrs.delete(:name)
        @value = component_attrs.delete(:value)
        @input_id = component_attrs.delete(:input_id)
        @label = component_attrs.delete(:label)
        @description = component_attrs.delete(:description)
        @checked = component_attrs.delete(:checked) || false
        @disabled = component_attrs.delete(:disabled) || false
        @hidden_field = component_attrs.delete(:hidden_field) || false
        @data = component_attrs.delete(:data) || {}
        @label_data = component_attrs.delete(:label_data) || {}
        @input_class = component_attrs.delete(:input_class)
        super(**component_attrs)
      end

      def view_template
        input(type: :hidden, name: name, value: '', disabled: disabled) if hidden_field
        label(**attrs) do
          input(**input_attrs)
          span(class: 'z-10 flex min-w-0 flex-col gap-1') do
            span(class: 'font-bold text-foreground') { label_text }
            span(class: 'text-sm text-on-surface-variant') { description } if description.present?
          end
        end
      end

      private

      attr_reader :type, :name, :value, :input_id, :description, :checked, :disabled, :hidden_field, :data, :label_data,
                  :input_class

      def label_text
        @label
      end

      def default_attrs
        {
          for: input_id,
          data: label_data,
          class: [
            'group relative flex min-h-11 cursor-pointer items-start gap-3 rounded-shape-xl border p-4',
            'border-outline-variant bg-surface-container-low transition-all state-layer',
            'hover:bg-surface-container-high',
            'has-[:checked]:border-primary has-[:checked]:bg-primary-container/40',
            'has-[:focus-visible]:outline has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-primary',
            'has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-50'
          ].join(' ')
        }
      end

      def input_attrs
        {
          type: type,
          name: name,
          value: value,
          id: input_id,
          checked: checked,
          disabled: disabled,
          class: input_classes,
          data: data
        }
      end

      def input_classes
        ['z-10 mt-0.5', input_class].compact.join(' ')
      end
    end
  end
end
