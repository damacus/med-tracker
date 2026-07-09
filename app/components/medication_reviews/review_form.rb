# frozen_string_literal: true

module Components
  module MedicationReviews
    class ReviewForm < Components::Base
      def initialize(prompt:)
        @prompt = prompt
        super()
      end

      def view_template
        form_with(model: prompt, url: medication_review_prompt_path(prompt), method: :patch,
                  class: 'space-y-4 border-t border-border pt-5 md:border-l md:border-t-0 md:pl-6 md:pt-0') do
          m3_heading(level: 4, size: '4', class: 'font-bold') { t('medication_reviews.form.title') }
          render_errors
          render_status_field
          render_practitioner_fields
          render_note_field
          m3_button(type: :submit, variant: :filled, class: 'w-full') { t('medication_reviews.form.save') }
        end
      end

      private

      attr_reader :prompt

      def render_errors
        return if prompt.errors.empty?

        div(class: 'rounded-shape-sm bg-error-container p-3 text-sm text-on-error-container', role: 'alert') do
          prompt.errors.full_messages.each { |message| p { message } }
        end
      end

      def render_status_field
        field('status', t('medication_reviews.form.status')) do |id|
          m3_select(name: 'medication_review_prompt[status]', id: id, size: :sm) do
            editable_statuses.each do |status|
              option(value: status, selected: prompt.status == status) do
                t("medication_reviews.statuses.#{status}")
              end
            end
          end
        end
      end

      def render_practitioner_fields
        div(class: 'grid gap-4 sm:grid-cols-2 md:grid-cols-1 lg:grid-cols-2') do
          field('practitioner_role', t('medication_reviews.form.practitioner_role')) do |id|
            m3_select(name: 'medication_review_prompt[practitioner_role]', id: id, size: :sm) do
              option(value: '') { t('medication_reviews.form.select_role') }
              practitioner_roles.each do |role|
                option(value: role, selected: prompt.practitioner_role == role) { role }
              end
            end
          end
          field('reviewed_on', t('medication_reviews.form.reviewed_on')) do |id|
            m3_input(type: :date, name: 'medication_review_prompt[reviewed_on]', id: id,
                     value: prompt.reviewed_on, class: 'h-9 min-h-9 px-3 py-2 text-sm')
          end
        end
        field('practitioner_name', t('medication_reviews.form.practitioner_name')) do |id|
          m3_input(type: :text, name: 'medication_review_prompt[practitioner_name]', id: id,
                   value: prompt.practitioner_name, class: 'h-9 min-h-9 px-3 py-2 text-sm')
        end
      end

      def render_note_field
        field('review_note', t('medication_reviews.form.review_note')) do |id|
          render RubyUI::Textarea.new(
            name: 'medication_review_prompt[review_note]',
            id: id,
            rows: 3,
            class: 'min-h-20 resize-y'
          ) { prompt.review_note.to_s }
        end
      end

      def field(name, label_text)
        id = "medication_review_prompt_#{prompt.id}_#{name}"
        div(class: 'space-y-1.5') do
          label(for: id, class: 'block text-xs font-bold text-on-surface-variant') { label_text }
          yield id
        end
      end

      def editable_statuses
        MedicationReviewPrompt::STATUSES - ['hidden_low_signal']
      end

      def practitioner_roles
        %w[Pharmacist Nurse GP Prescriber Other]
      end
    end
  end
end
