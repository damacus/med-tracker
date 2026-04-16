# frozen_string_literal: true

module Components
  module Medications
    class SearchComponent < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :current_category, :categories, :locations, :current_location_id

      def initialize(current_category: nil, categories: [], locations: [], current_location_id: nil)
        @current_category = current_category
        @categories = categories
        @locations = locations
        @current_location_id = current_location_id
        super()
      end

      def view_template
        return if locations.empty? && categories.empty?

        div(class: 'mb-12 grid grid-cols-1 md:grid-cols-2 gap-6 max-w-3xl') do
          form_with(url: medications_path, method: :get, class: 'contents', data: { controller: 'form-submit' }) do
            render_filter(location_filter_config) if locations.any?
            render_filter(category_filter_config) if categories.any?
          end
        end
      end

      private

      def render_filter(config)
        div(class: 'space-y-2') do
          render_filter_label(config)
          render_filter_combobox(config)
        end
      end

      def render_filter_label(config)
        render RubyUI::FormFieldLabel.new(
          for: config.fetch(:label_for),
          class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
        ) { config.fetch(:label_text) }
      end

      def render_filter_combobox(config)
        render RubyUI::Combobox.new(class: 'w-full') do
          render RubyUI::ComboboxTrigger.new(placeholder: config.fetch(:trigger_placeholder))
          render_filter_popover(config)
        end
      end

      def render_filter_popover(config)
        render RubyUI::ComboboxPopover.new do
          render RubyUI::ComboboxSearchInput.new(placeholder: config.fetch(:search_placeholder))
          render_filter_list(config)
        end
      end

      def render_filter_list(config)
        render RubyUI::ComboboxList.new do
          render(RubyUI::ComboboxEmptyState.new { config.fetch(:empty_text) })
          render_filter_option(
            name: config.fetch(:name),
            value: '',
            checked: config.fetch(:current_value).blank?,
            label: config.fetch(:all_text)
          )
          render_filter_options(config)
        end
      end

      def render_filter_options(config)
        config.fetch(:options).each do |option|
          option_value = option_value_for(config, option)
          render_filter_option(
            name: config.fetch(:name),
            value: option_value,
            checked: config.fetch(:current_value) == option_value,
            label: option_label_for(config, option)
          )
        end
      end

      def location_filter_config
        {
          label_for: 'inventory_location_trigger',
          label_text: t('medications.show.location'),
          trigger_placeholder: current_location_name ||
            t('medications.index.all_locations', default: 'All locations'),
          search_placeholder: t('medications.index.search_locations', default: 'Filter locations'),
          empty_text: t('medications.index.no_locations_found', default: 'No locations found'),
          name: 'location_id',
          current_value: current_location_id,
          all_text: t('medications.index.all_locations', default: 'All locations'),
          options: locations,
          value_key: :id,
          label_key: :name
        }
      end

      def category_filter_config
        {
          label_for: 'inventory_category_trigger',
          label_text: 'Category',
          trigger_placeholder: current_category.presence || t('medications.index.all'),
          search_placeholder: t('forms.medications.filter_categories'),
          empty_text: t('forms.medications.no_categories_found'),
          name: 'category',
          current_value: current_category,
          all_text: t('medications.index.all'),
          options: categories
        }
      end

      def option_value_for(config, option)
        value_key = config[:value_key]
        value_key ? option.public_send(value_key) : option
      end

      def option_label_for(config, option)
        label_key = config[:label_key]
        label_key ? option.public_send(label_key) : option
      end

      def render_filter_option(name:, value:, checked:, label:)
        render RubyUI::ComboboxItem.new do
          render RubyUI::ComboboxRadio.new(
            name: name,
            value: value,
            checked: checked,
            data: { action: 'change->form-submit#submitForm' }
          )
          span { label }
        end
      end

      def current_location_name
        return nil if current_location_id.blank?

        locations.find { |location| location.id == current_location_id }&.name
      end
    end
  end
end
