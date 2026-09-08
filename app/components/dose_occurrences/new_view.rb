module Components
  module DoseOccurrences
    class NewView < Components::Base
      def initialize(source:, occurrences:, values: {}, error: nil)
        @source = source
        @occurrences = occurrences
        @values = values
        @error = error
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-xl space-y-6 px-4 py-8') do
          m3_heading(level: 1, variant: :headline_medium) { t('dose_outcomes.title') }
          m3_text(variant: :title_medium) { @source.person.name }
          m3_text(variant: :body_large) { @source.medication.display_name }
          m3_text(variant: :body_medium) { t('dose_outcomes.description') }
          div(role: 'alert', class: 'rounded-shape-md bg-error-container p-4') { @error } if @error
          if @occurrences.empty?
            m3_text { t('dose_outcomes.empty') }
          else
            render_form
          end
          m3_link(href: dashboard_path(dashboard_person_id: @source.person_id), variant: :text) do
            t('dose_outcomes.cancel')
          end
        end
      end

      private

      def render_form
        form_with(url: schedule_dose_occurrences_path(@source), method: :post,
                  class: 'space-y-5', data: { turbo_frame: '_top' }) do
          render_dose_field
          render_reason_field
          render_note_field
          m3_button(type: :submit, variant: :filled, class: 'w-full') { t('dose_outcomes.confirm') }
        end
      end

      def render_dose_field
        field('dose', t('dose_outcomes.dose')) do
          m3_select(id: 'outcome_dose', name: 'dose_occurrence[key]') do
            @occurrences.each do |row|
              option(value: row.key) do
                time = row.scheduled_at&.strftime('%H:%M') || t('dashboard.routine.anytime')
                t('dose_outcomes.slot', position: row.position, date: row.window_starts_on.iso8601, time: time)
              end
            end
          end
        end
      end

      def render_reason_field
        field('reason', t('dose_outcomes.reason')) do
          m3_select(id: 'outcome_reason', name: 'dose_occurrence[reason]') do
            option(value: '') { t('dose_outcomes.unspecified') }
            MedicationDoseOccurrence::REASONS.each do |reason|
              option(value: reason, selected: @values[:reason] == reason) { t("dashboard.outcomes.reasons.#{reason}") }
            end
          end
        end
      end

      def render_note_field
        field('note', t('dose_outcomes.note')) do
          render RubyUI::Textarea.new(id: 'outcome_note', name: 'dose_occurrence[note]', rows: 3,
                                      maxlength: 2000) { @values[:note].to_s }
        end
      end

      def field(name, text)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: "outcome_#{name}") { text }
          yield
        end
      end
    end
  end
end
