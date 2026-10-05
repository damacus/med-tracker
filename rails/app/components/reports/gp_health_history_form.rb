# frozen_string_literal: true

module Components
  module Reports
    class GpHealthHistoryForm < Components::Base
      def initialize(report:, people:, selected_person_id:)
        @report = report
        @people = people
        @selected_person_id = selected_person_id
        @include_medication_takes = report.include_medication_takes
        super()
      end

      def view_template
        form(
          action: @report.view_context.health_history_report_path({}), method: :get,
          class: 'flex flex-col gap-2 sm:flex-row sm:items-end'
        ) do
          render_person_field
          render_medication_takes_field
          m3_button(type: 'submit', variant: :outlined, size: :sm) { @report.t('reports.index.download_pdf') }
        end
      end

      private

      def render_person_field
        div(class: 'flex flex-col gap-1') do
          label(for: 'health_history_person_id', class: label_classes) do
            @report.t('reports.index.gp_person_label')
          end
          m3_select(name: 'person_id', id: 'health_history_person_id', size: :sm) do
            option(value: '') { @report.t('reports.index.gp_person_prompt') }
            @people.each { |person| render_person_option(person) }
          end
        end
      end

      def render_person_option(person)
        option(value: person.id, selected: person.id.to_s == @selected_person_id.to_s) { person.name }
      end

      def render_medication_takes_field
        div(class: 'flex items-center gap-2') do
          input(
            type: 'checkbox',
            name: 'include_medication_takes',
            id: 'include_medication_takes',
            value: '1',
            checked: @include_medication_takes,
            class: 'h-4 w-4 rounded border-primary focus-visible:outline-none focus-visible:ring-2 ' \
                   'focus-visible:ring-ring'
          )
          label(for: 'include_medication_takes', class: 'text-sm text-on-surface-variant') do
            @report.t('reports.index.include_medication_takes')
          end
        end
      end

      def label_classes = 'text-xs font-semibold uppercase tracking-wider text-on-surface-variant'
    end
  end
end
