# frozen_string_literal: true

module Components
  module Reports
    class FilterForm < Components::Base
      def initialize(action_path:, people:, selected_person_id:, start_date:, end_date:)
        @action_path = action_path
        @people = people
        @selected_person_id = selected_person_id
        @start_date = start_date
        @end_date = end_date
        super()
      end

      def view_template
        form(action: @action_path, method: :get, class: form_classes) do
          render_person_field
          render_date_field(name: 'start_date', label: translate('start_date_label'), value: @start_date)
          render_date_field(name: 'end_date', label: translate('end_date_label'), value: @end_date)
          m3_button(
            type: 'submit',
            class: 'rounded-xl shadow-elevation-1',
            'aria-label': translate('apply_filters_aria_label')
          ) do
            render Icons::ChevronRight.new(size: 20)
          end
        end
      end

      private

      def form_classes
        'flex flex-wrap items-end gap-3 rounded-[1.5rem] border border-border/70 bg-popover p-4 shadow-elevation-1'
      end

      def render_person_field
        div(class: 'flex flex-col gap-1') do
          label(for: 'person_id', class: label_classes) { translate('person_filter_label') }
          m3_select(name: 'person_id', id: 'person_id', size: :sm) do
            option(value: '', selected: @selected_person_id.blank?) { translate('all_people') }
            @people.each { |person| render_person_option(person) }
          end
        end
      end

      def render_person_option(person)
        option(value: person.id, selected: person.id.to_s == @selected_person_id.to_s) { person.name }
      end

      def render_date_field(name:, label:, value:)
        div(class: 'flex flex-col gap-1') do
          label(for: name, class: label_classes) { label }
          input(type: 'date', name: name, id: name, value: value, class: input_classes(:input))
        end
      end

      def label_classes
        'text-xs font-semibold uppercase tracking-wider text-on-surface-variant'
      end

      def input_classes(type)
        [
          "form-#{type}",
          'rounded-shape-sm border-border bg-background text-sm text-foreground',
          'focus:border-primary focus:ring-primary'
        ].join(' ')
      end

      def translate(key)
        I18n.t("reports.index.#{key}")
      end
    end
  end
end
