# frozen_string_literal: true

module RubyUI
  class ToggleGroup < Base
    SPACING_GAP = { 0 => nil, 1 => 'gap-1', 2 => 'gap-2', 3 => 'gap-3', 4 => 'gap-4' }.freeze
    VALID_TYPES = %i[single multiple].freeze
    VALID_ORIENTATIONS = %i[horizontal vertical].freeze

    def initialize(**attrs)
      @type = attrs.delete(:type) { :single }.to_sym
      raise ArgumentError, 'type must be :single or :multiple' unless VALID_TYPES.include?(@type)

      @orientation = attrs.delete(:orientation) { :horizontal }.to_sym
      unless VALID_ORIENTATIONS.include?(@orientation)
        raise ArgumentError, 'orientation must be :horizontal or :vertical'
      end

      spacing = attrs.delete(:spacing) { 0 }
      raise ArgumentError, 'spacing must be an Integer 0..4' unless spacing.is_a?(Integer) && (0..4).cover?(spacing)

      @name = attrs.delete(:name)
      @value = attrs.delete(:value)
      @variant = attrs.delete(:variant) { :default }.to_sym
      @size = attrs.delete(:size) { :default }.to_sym
      @disabled = attrs.delete(:disabled) { false }
      @spacing = spacing
      super
    end

    def view_template(&)
      div(**attrs) do
        yield(self)
        render_hidden_inputs
      end
    end

    def item_context
      {
        type: @type,
        variant: @variant,
        size: @size,
        disabled: @disabled,
        selected_values: selected_values,
        spacing: @spacing,
        orientation: @orientation
      }
    end

    def toggle_group_item(**, &)
      render(RubyUI::ToggleGroupItem.new(group_context: item_context, **), &)
    end

    private

    def selected_values
      case @type
      when :single then @value.nil? ? [] : [@value.to_s]
      when :multiple then Array(@value).map(&:to_s)
      end
    end

    def render_hidden_inputs
      return unless @name

      if @type == :single
        input(
          type: 'hidden',
          name: @name,
          value: selected_values.first.to_s,
          data: { 'ruby-ui--toggle-group-target': 'input' }
        )
      else
        selected_values.each do |v|
          input(
            type: 'hidden',
            name: "#{@name}[]",
            value: v,
            data: { 'ruby-ui--toggle-group-target': 'input' }
          )
        end
      end
    end

    def default_attrs
      {
        role: @type == :single ? 'radiogroup' : 'group',
        data: {
          controller: 'ruby-ui--toggle-group',
          'ruby-ui--toggle-group-type-value': @type.to_s,
          'ruby-ui--toggle-group-name-value': @name.to_s,
          orientation: @orientation.to_s,
          spacing: @spacing.to_s
        },
        class: container_classes
      }
    end

    def container_classes
      base = if @orientation == :vertical
               'flex w-fit flex-col items-stretch rounded-md'
             else
               'flex w-fit items-center rounded-md'
             end

      [
        base,
        SPACING_GAP[@spacing],
        @spacing.zero? && @variant == :outline ? 'shadow-xs' : nil
      ].compact
    end
  end
end
