module Components
  module DoseOccurrences
    class EditView < Components::Base
      def initialize(source:, outcome:, medications:, can_reopen:, error: nil)
        @source = source
        @outcome = outcome
        @medications = medications
        @can_reopen = can_reopen
        @error = error
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-xl space-y-6 px-4 py-8') do
          m3_heading(level: 1, variant: :headline_medium) { t('dose_outcomes.correction_title') }
          div(class: 'space-y-2') do
            m3_text(variant: :title_medium) { @source.person.name }
            m3_text(variant: :body_large) { @source.medication.display_name }
            m3_text { window_label }
          end
          div(role: 'alert', class: 'rounded-shape-md bg-error-container p-4') { @error } if @error
          if @outcome.not_taken?
            render_decision
            render_reopen if @can_reopen
            render_take
          else
            m3_text { t('dose_outcomes.resolved_elsewhere') }
          end
          m3_link(href: dashboard_path(dashboard_person_id: @source.person_id), variant: :text) do
            t('dose_outcomes.cancel')
          end
        end
      end

      private

      def window_label
        return @outcome.window_starts_on.iso8601 if @outcome.window_ends_on == @outcome.window_starts_on

        t('dose_outcomes.cycle_slot', position: @outcome.position, start_date: @outcome.window_starts_on.iso8601,
                                      end_date: @outcome.window_ends_on.iso8601)
      end

      def correction_path
        return schedule_dose_occurrence_path(@source, @outcome) if @source.is_a?(::Schedule)

        person_medication_dose_occurrence_path(@source, @outcome)
      end

      def default_taken_at
        return unless Date.current.between?(@outcome.window_starts_on, @outcome.window_ends_on)

        Time.current.strftime('%Y-%m-%dT%H:%M')
      end

      def render_decision
        div(class: 'space-y-1') do
          m3_text { t('dashboard.outcomes.not_taken') }
          m3_text { t("dashboard.outcomes.reasons.#{@outcome.reason}") } if @outcome.reason.present?
          m3_text { @outcome.note } if @outcome.note.present?
        end
      end

      def render_reopen
        correction_form('reopen') do
          m3_text { t('dose_outcomes.reopen_description') }
          m3_button(type: :submit, variant: :outlined) { t('dose_outcomes.reopen') }
        end
      end

      def render_take
        correction_form('take') do
          m3_text { t('dose_outcomes.take_description') }
          input(type: :hidden, name: 'dose_occurrence[client_uuid]', value: SecureRandom.uuid)
          render_time
          render_stock
          m3_button(type: :submit, variant: :filled) { t('dose_outcomes.take') }
        end
      end

      def correction_form(resolution)
        form_with(url: correction_path, method: :patch,
                  class: 'space-y-4 rounded-shape-xl border border-border p-4', data: { turbo_frame: '_top' }) do
          input(type: :hidden, name: 'dose_occurrence[resolution]', value: resolution)
          input(type: :hidden, name: 'dose_occurrence[etag]', value: Api::RecordEtag.for(@outcome))
          yield
        end
      end

      def render_time
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: 'correction_taken_at') { t('dose_outcomes.taken_at') }
          m3_input(type: 'datetime-local', id: 'correction_taken_at', name: 'dose_occurrence[taken_at]',
                   required: true, max: Time.current.strftime('%Y-%m-%dT%H:%M'),
                   value: default_taken_at)
        end
      end

      def render_stock
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(for: 'correction_stock') { t('dose_outcomes.stock') }
          m3_select(id: 'correction_stock', name: 'dose_occurrence[taken_from_medication_id]', required: true) do
            @medications.each do |medication|
              option(value: medication.id) { [medication.location&.name, medication.display_name].compact.join(' · ') }
            end
          end
        end
      end
    end
  end
end
