# frozen_string_literal: true

module Components
  module PersonMedications
    # Modal component for person medication form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :person_medication, :person, :medications, :editing, :back_path

      # rubocop:disable Metrics/ParameterLists
      def initialize(person_medication:, person:, medications:, title: nil, editing: false, back_path: nil)
        # rubocop:enable Metrics/ParameterLists
        @person_medication = person_medication
        @person = person
        @medications = medications
        @editing = editing
        @back_path = back_path
        @explicit_title = title
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: dialog_size) do
              DialogHeader do
                if back_path
                  a(
                    href: back_path,
                    data: { turbo_frame: 'modal' },
                    class: 'inline-flex items-center text-sm text-muted-foreground hover:text-foreground ' \
                           'transition-colors mb-2 no-underline'
                  ) do
                    plain t('medication_workflow.back')
                  end
                end
                DialogTitle { title }
                DialogDescription { t('person_medications.modal.subtitle') }
              end
              DialogMiddle do
                render_form
              end
            end
          end
        end
      end

      private

      def render_form
        form_with(
          model: person_medication,
          url: form_url,
          method: editing ? :patch : :post,
          class: 'space-y-6',
          data: {
            controller: 'person-medication-form',
            person_type: person.person_type,
            person_medication_form_current_step_value: workflow_initial_step,
            person_medication_form_translations_value: {
              chooseMedication: t('person_medications.form.workflow.choose_medication_title'),
              chooseDose: t('person_medications.form.workflow.choose_dose_title'),
              selectDose: t('person_medications.form.select_dose'),
              noDosesAvailable: t('person_medications.form.no_doses_available')
            }.to_json
          }
        ) do
          render_form_fields
          render_actions
        end
      end

      def form_url
        if editing
          person_person_medication_path(person, person_medication)
        else
          person_person_medications_path(person)
        end
      end

      def render_form_fields
        render FormFields.new(
          person_medication: person_medication,
          medications: medications,
          editing: editing,
          workflow: !editing
        )
      end

      def render_actions
        div(class: 'pt-4') do
          if editing
            div(class: 'flex items-center justify-end gap-6') do
              Button(
                variant: :ghost,
                data: { action: 'click->ruby-ui--dialog#dismiss' }
              ) { t('person_medications.form.cancel') }
              Button(type: :submit, variant: :primary) { t('person_medications.form.save_changes_button') }
            end
          else
            div(class: 'flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-end sm:gap-6') do
              div(class: 'order-2 sm:order-1 sm:mr-auto') do
                Button(
                  variant: :ghost,
                  size: :xl,
                  class: 'w-full justify-center sm:w-auto',
                  data: { action: 'click->ruby-ui--dialog#dismiss' }
                ) { t('person_medications.form.cancel') }
              end
              div(class: 'order-1 flex w-full items-center gap-3 sm:order-2 sm:w-auto') do
                Button(
                  type: :button,
                  variant: :outline,
                  size: :xl,
                  class: 'hidden min-w-0 flex-1 sm:min-w-28 sm:flex-none',
                  data: {
                    action: 'click->person-medication-form#prevStep',
                    person_medication_form_target: 'prevButton'
                  }
                ) { t('person_medications.form.back') }
                Button(
                  type: :button,
                  variant: :primary,
                  size: :xl,
                  class: 'min-w-0 flex-1 sm:min-w-28 sm:flex-none',
                  data: {
                    action: 'click->person-medication-form#nextStep',
                    person_medication_form_target: 'nextButton'
                  }
                ) { t('person_medications.form.next') }
                Button(
                  type: :submit,
                  variant: :primary,
                  size: :xl,
                  class: 'hidden min-w-0 flex-1 sm:min-w-28 sm:flex-none',
                  data: { person_medication_form_target: 'submitButton' }
                ) { t('person_medications.form.add_medication_button') }
              end
            end
          end
        end
      end

      def title
        @explicit_title || default_title
      end

      def default_title
        if editing
          t('person_medications.modal.edit_title', person: person.name)
        else
          t('person_medications.modal.new_title', person: person.name)
        end
      end

      def workflow_initial_step
        return 1 if editing
        return 2 if person_medication.errors[:dose_amount].any? || person_medication.errors[:dose_unit].any?
        return 3 if person_medication.errors.any?
        return 2 if person_medication.medication_id.present?

        1
      end

      def dialog_size
        editing ? :xl : :md
      end
    end
  end
end
