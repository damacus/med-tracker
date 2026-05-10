# frozen_string_literal: true

module Components
  module MedicationAssignments
    class Form < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :assignment, :person, :medications, :modal, :back_path

      def initialize(assignment:, person:, medications:, modal: false, back_path: nil)
        @assignment = assignment
        @person = person
        @medications = medications
        @modal = modal
        @back_path = back_path
        super()
      end

      def view_template
        form_with(
          scope: :medication_assignment,
          url: person_medication_assignments_path(person),
          method: :post,
          class: "space-y-6",
          data: form_data
        ) do
          render_errors if assignment.errors.any?
          render_workflow
          render_actions
        end
      end

      private

      def form_data
        {
          controller: "medication-assignment-form",
          person_type: person.person_type,
          medication_assignment_form_options_value: medication_options_payload.to_json,
          medication_assignment_form_current_step_value: initial_step,
          medication_assignment_form_start_date_value: start_date.to_s,
          medication_assignment_form_end_date_value: end_date.to_s,
          medication_assignment_form_translations_value: translations_payload.to_json
        }
      end

      def render_errors
        Alert(variant: :destructive, class: "mb-6") do
          AlertTitle do
            t("person_medications.form.validation_errors", count: assignment.errors.count)
          end

          AlertDescription do
            ul(class: "my-2 ml-6 list-disc [&>li]:mt-1") do
              assignment.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end
      end

      def render_workflow
        div(class: "space-y-6") do
          div(class: "flex items-center justify-center gap-2") do
            [1, 2, 3].each do |step_number|
              div(
                class: workflow_progress_classes(step_number),
                data: {medication_assignment_form_target: "stepIndicator"}
              )
            end
          end

          render_workflow_step(
            1,
            title: t("person_medications.form.workflow.choose_medication_title"),
            description: t("person_medications.form.workflow.choose_medication_description")
          ) do
            render_medication_field
          end

          render_workflow_step(
            2,
            title: t("person_medications.form.workflow.choose_dose_title"),
            description: t("schedules.form.choose_one_dose")
          ) do
            render_selection_summary
            render_dose_field
          end

          render_workflow_step(
            3,
            title: t("medication_assignments.form.review_title"),
            description: t("medication_assignments.form.review_description")
          ) do
            render_selection_summary(show_dose: true)
            render_review_panel
          end
        end
      end

      def render_workflow_step(step_number, title:, description:, &)
        div(
          class: workflow_step_classes(step_number),
          data: {medication_assignment_form_target: "stepPanel", step: step_number}
        ) do
          div(class: "space-y-1") do
            m3_heading(level: 2, size: "4", class: "font-semibold") { title }
            m3_text(size: "2", class: "text-on-surface-variant") { description }
          end

          div(class: "space-y-4", &)
        end
      end

      def render_medication_field
        FormField do
          FormFieldLabel(for: "medication_assignment_medication_id_trigger") do
            t("person_medications.form.medication")
          end

          render(RubyUI::Combobox.new(class: "w-full")) do
            render(
              RubyUI::ComboboxTrigger.new(
                placeholder: selected_medication_name || t("person_medications.form.select_medication")
              )
            )

            render(RubyUI::ComboboxPopover.new) do
              render(RubyUI::ComboboxSearchInput.new(placeholder: t("person_medications.form.search_medications")))

              render(RubyUI::ComboboxList.new) do
                render(RubyUI::ComboboxEmptyState.new { t("person_medications.form.no_medications_found") })

                medications.each do |medication|
                  render(RubyUI::ComboboxItem.new) do
                    render(
                      RubyUI::ComboboxRadio.new(
                        name: "medication_assignment[medication_id]",
                        id: "medication_assignment_medication_id_#{medication.id}",
                        value: medication.id,
                        checked: assignment.medication_id == medication.id,
                        required: true,
                        data: {
                          text: medication.name,
                          medication_assignment_form_target: "medicationSelect",
                          action: "change->medication-assignment-form#updateMedication"
                        }
                      )
                    )
                    span { medication.name }
                  end
                end
              end
            end
          end

          FormFieldHint { t("person_medications.form.medication_hint") }
        end
      end

      def render_dose_field
        FormField do
          FormFieldLabel(for: "medication_assignment_dose_option") do
            plain(t("person_medications.form.dose"))
            span(class: "text-destructive ml-0.5") { " *" }
          end

          input(
            type: :hidden,
            name: "medication_assignment[source_dosage_option_id]",
            value: assignment.source_dosage_option_id,
            data: {medication_assignment_form_target: "sourceDosageOptionIdInput"}
          )
          input(
            type: :hidden,
            name: "medication_assignment[dose_amount]",
            value: decimal_string(assignment.dose_amount),
            data: {medication_assignment_form_target: "doseAmountInput"}
          )
          input(
            type: :hidden,
            name: "medication_assignment[dose_unit]",
            value: assignment.dose_unit,
            data: {medication_assignment_form_target: "doseUnitInput"}
          )
          select(
            id: "medication_assignment_dose_option",
            name: "medication_assignment[dose_option]",
            required: true,
            disabled: assignment.medication_id.blank?,
            class: "w-full rounded-md border border-outline bg-background px-3 py-2 text-sm",
            data: {
              medication_assignment_form_target: "doseOptionInput",
              action: "change->medication-assignment-form#selectDose"
            }
          ) do
            option(value: "") do
              if assignment.medication_id.present?
                t("person_medications.form.select_dose")
              else
                t("person_medications.form.select_medication_first")
              end
            end
          end

          FormFieldHint { t("person_medications.form.dose_hint") }
        end
      end

      def render_selection_summary(show_dose: false)
        div(class: selection_summary_layout_classes(show_dose)) do
          render_summary_card(
            t("person_medications.form.medication"),
            selected_medication_name || t("person_medications.form.workflow.choose_medication_title"),
            "selectedMedicationName"
          )
          if show_dose
            render_summary_card(
              t("person_medications.form.dose"),
              t("person_medications.form.workflow.choose_dose_title"),
              "selectedDoseName"
            )
          end
        end
      end

      def render_summary_card(label, value, target)
        div(class: "rounded-shape-xl border border-border/60 bg-popover px-4 py-3 shadow-elevation-1") do
          m3_text(
            size: "1",
            weight: "medium",
            class: "uppercase tracking-[0.2em] text-on-surface-variant"
          ) { label }
          m3_text(size: "3", weight: "semibold", data: {medication_assignment_form_target: target}) { value }
        end
      end

      def render_review_panel
        div(class: "grid grid-cols-1 gap-3 md:grid-cols-2") do
          render_review_item(t("medication_assignments.form.frequency"), "reviewFrequency")
          render_review_item(t("schedules.form.max_doses_per_cycle"), "reviewMaxDoses")
          render_review_item(t("schedules.form.min_hours_between_doses"), "reviewMinHours")
          render_review_item(t("schedules.form.dose_cycle"), "reviewDoseCycle")
          render_review_item(t("medication_assignments.form.schedule_type"), "reviewScheduleType")
          render_review_item(t("medication_assignments.form.active_dates"), "reviewActiveDates")
        end
      end

      def render_review_item(label, target)
        div(class: "rounded-shape-xl border border-outline-variant/60 bg-surface-container-low px-4 py-3") do
          m3_text(
            size: "1",
            weight: "medium",
            class: "uppercase tracking-[0.2em] text-on-surface-variant"
          ) { label }
          m3_text(
            size: "2",
            weight: "semibold",
            data: {medication_assignment_form_target: target}
          ) do
            t("medication_assignments.form.not_set")
          end
        end
      end

      def render_actions
        div(class: "pt-4") do
          div(class: "flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-end sm:gap-6") do
            div(class: "order-2 sm:order-1 sm:mr-auto") { render_cancel_action }
            div(class: "order-1 flex w-full items-center gap-3 sm:order-2 sm:w-auto") do
              m3_button(
                type: :button,
                variant: :outlined,
                size: :xl,
                class: "hidden min-w-0 flex-1 sm:min-w-28 sm:flex-none",
                data: {
                  action: "click->medication-assignment-form#prevStep",
                  medication_assignment_form_target: "prevButton"
                }
              ) { t("person_medications.form.back") }
              m3_button(
                type: :button,
                variant: :filled,
                size: :xl,
                class: "min-w-0 flex-1 sm:min-w-28 sm:flex-none",
                data: {
                  action: "click->medication-assignment-form#nextStep",
                  medication_assignment_form_target: "nextButton"
                }
              ) { t("person_medications.form.next") }
              m3_button(
                type: :submit,
                variant: :filled,
                size: :xl,
                class: "hidden min-w-0 flex-1 sm:min-w-28 sm:flex-none",
                data: {medication_assignment_form_target: "submitButton"}
              ) { t("person_medications.form.add_medication_button") }
            end
          end
        end
      end

      def render_cancel_action
        if modal
          m3_link(
            href: person_path(person),
            variant: :text,
            size: :xl,
            class: "w-full justify-center sm:w-auto",
            data: {turbo_frame: "_top"}
          ) { t("person_medications.form.cancel") }
        else
          m3_link(
            href: person_path(person),
            variant: :text,
            size: :xl,
            class: "w-full justify-center sm:w-auto"
          ) { t("person_medications.form.cancel") }
        end
      end

      def medication_options_payload
        medications.to_h do |medication|
          [medication.id.to_s, medication_payload(medication)]
        end
      end

      def medication_payload(medication)
        {
          name: medication.name,
          default_schedule_type: medication.default_schedule_type,
          schedule_type_label: schedule_type_label(medication.default_schedule_type),
          dose_options: dose_options_for(medication)
        }
      end

      def dose_options_for(medication)
        options = medication.dose_options_payload.map(&:deep_stringify_keys)
        return options if options.any?
        return [] if medication.dosage_amount.blank? || medication.dosage_unit.blank?

        [
          {
            "amount" => decimal_string(medication.dosage_amount),
            "unit" => medication.dosage_unit,
            "frequency" => medication.default_schedule_type_prn? ? "As needed" : "As directed",
            "default_dose_cycle" => "daily",
            "option_value" => "#{decimal_string(medication.dosage_amount)}|#{medication.dosage_unit}"
          }
        ]
      end

      def schedule_type_label(schedule_type)
        return t("forms.medications.wizard.dose.schedule_types.prn.title") if schedule_type == "prn"

        schedule_type.to_s.humanize
      end

      def translations_payload
        {
          chooseMedication: t("person_medications.form.workflow.choose_medication_title"),
          chooseDose: t("person_medications.form.workflow.choose_dose_title"),
          selectDose: t("person_medications.form.select_dose"),
          noDosesAvailable: t("person_medications.form.no_doses_available"),
          notSet: t("medication_assignments.form.not_set")
        }
      end

      def selected_medication_name
        return nil if assignment.medication_id.blank?

        medications.find { |medication| medication.id == assignment.medication_id }&.name
      end

      def initial_step
        return 2 if assignment.errors[:source_dosage_option].any?
        return 2 if assignment.medication_id.present?

        1
      end

      def workflow_step_classes(step_number)
        classes = ["space-y-4"]
        classes << "hidden" if initial_step != step_number
        classes.join(" ")
      end

      def workflow_progress_classes(step_number)
        classes = %w[h-2 w-10 rounded-full transition-colors]
        classes << (step_number <= initial_step ? "bg-foreground" : "bg-primary/15")
        classes.join(" ")
      end

      def selection_summary_layout_classes(show_dose)
        show_dose ? "grid grid-cols-1 md:grid-cols-2 gap-3" : "max-w-md"
      end

      def decimal_string(value)
        return if value.blank?

        value.is_a?(BigDecimal) ? value.to_s("F") : value.to_s
      end

      def start_date
        Time.zone.today
      end

      def end_date
        1.month.from_now.to_date
      end
    end
  end
end
