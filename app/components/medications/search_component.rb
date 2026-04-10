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
            render_location_filter if locations.any?
            render_category_filter if categories.any?
          end
        end
      end

      private

      def render_location_filter
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'inventory_location_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('medications.show.location') }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: current_location_name || t('medications.index.all_locations', default: 'All locations')
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(
                placeholder: t('medications.index.search_locations', default: 'Filter locations')
              )

              render RubyUI::ComboboxList.new do
                render RubyUI::ComboboxEmptyState.new do
                  t('medications.index.no_locations_found', default: 'No locations found')
                end

                render RubyUI::ComboboxItem.new do
                  render RubyUI::ComboboxRadio.new(
                    name: 'location_id',
                    value: '',
                    checked: current_location_id.blank?,
                    data: { action: 'change->form-submit#submitForm' }
                  )
                  span { t('medications.index.all_locations', default: 'All locations') }
                end

                locations.each do |location|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'location_id',
                      value: location.id,
                      checked: current_location_id == location.id,
                      data: { action: 'change->form-submit#submitForm' }
                    )
                    span { location.name }
                  end
                end
              end
            end
          end
        end
      end

      def render_category_filter
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'inventory_category_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { 'Category' }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: current_category.presence || t('medications.index.all')
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(
                placeholder: t('forms.medications.filter_categories')
              )

              render RubyUI::ComboboxList.new do
                render RubyUI::ComboboxEmptyState.new do
                  t('forms.medications.no_categories_found')
                end

                render RubyUI::ComboboxItem.new do
                  render RubyUI::ComboboxRadio.new(
                    name: 'category',
                    value: '',
                    checked: current_category.blank?,
                    data: { action: 'change->form-submit#submitForm' }
                  )
                  span { t('medications.index.all') }
                end

                categories.each do |category|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'category',
                      value: category,
                      checked: current_category == category,
                      data: { action: 'change->form-submit#submitForm' }
                    )
                    span { category }
                  end
                end
              end
            end
          end
        end
      end

      def current_location_name
        return nil if current_location_id.blank?

        locations.find { |location| location.id == current_location_id }&.name
      end
    end
  end
end
