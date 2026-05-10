# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepWarnings < Components::Base
        include FieldHelpers

        attr_reader :medication, :locations, :ai_medication_help_enabled

        def initialize(medication:, locations: [], ai_medication_help_enabled: false)
          @medication = medication
          @locations = locations
          @ai_medication_help_enabled = ai_medication_help_enabled
          super()
        end

        def view_template
          div(class: "space-y-6") do
            div(class: "space-y-1 mb-2") do
              div(class: "flex items-center gap-2") do
                render(Icons::AlertCircle.new(size: 20, class: "text-on-error-container"))
                m3_heading(level: 3, size: "5", class: "font-bold tracking-tight text-foreground") do
                  t("forms.medications.warnings")
                end
              end

              m3_text(size: "2", class: "text-on-surface-variant") do
                "Add any safety warnings or important notes"
              end
            end

            render_warnings_field
            render_ai_medication_confirmation if ai_medication_help_enabled

            div(class: "rounded-shape-xl bg-warning-container border border-warning p-4") do
              div(class: "flex gap-3") do
                render(Icons::AlertCircle.new(size: 16, class: "text-on-warning-container mt-0.5 shrink-0"))
                m3_text(size: "2", class: "text-on-warning-container") do
                  "Warnings will be displayed prominently on the medication profile " \
                    "and when administering doses."
                end
              end
            end
          end
        end

        private

        def render_ai_medication_confirmation
          div(
            class: "hidden rounded-3xl border border-primary/30 bg-primary/5 p-4 space-y-3",
            data: {'ai-medication-help-target': "confirmationPanel"}
          ) do
            div(class: "space-y-1") do
              m3_heading(level: 4, size: "4", class: "font-bold text-foreground") { "Review AI suggestions" }
              m3_text(size: "2", class: "text-on-surface-variant") { ai_medication_confirmation_text }
            end

            div(class: "space-y-2", data: {'ai-medication-help-target': "sourceList"})
            label(class: "flex items-start gap-3 text-sm font-semibold text-foreground") do
              input(
                type: "checkbox",
                name: "ai_medication_suggestion_confirmed",
                value: "1",
                class: "mt-1 rounded border-outline",
                data: {'ai-medication-help-target': "confirmationInput"}
              )
              span { "I checked these suggestions against the packet, leaflet, or linked guidance." }
            end
          end
        end

        def ai_medication_confirmation_text
          "Suggested from trusted source text. Check against the packet, leaflet, or linked guidance before saving."
        end
      end
    end
  end
end
