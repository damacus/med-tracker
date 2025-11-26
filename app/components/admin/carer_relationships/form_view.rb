# frozen_string_literal: true

module Components
  module Admin
    module CarerRelationships
      class FormView < Components::Base
        include Phlex::Rails::Helpers::FormWith

        RELATIONSHIP_TYPES = [
          ['Parent', 'parent'],
          ['Family member', 'family_member'],
          ['Professional carer', 'professional_carer'],
          ['Self', 'self']
        ].freeze

        attr_reader :relationship, :carers, :patients, :url_helpers

        def initialize(relationship:, carers:, patients:, url_helpers:)
          @relationship = relationship
          @carers = carers
          @patients = patients
          @url_helpers = url_helpers
          super()
        end

        def view_template
          div(class: 'max-w-2xl mx-auto') do
            render_header
            render_form
          end
        end

        private

        def render_header
          header(class: 'mb-8') do
            h1(class: 'text-3xl font-semibold text-slate-900') do
              relationship.new_record? ? 'New Carer Relationship' : 'Edit Carer Relationship'
            end
            p(class: 'text-slate-600 mt-2') { 'Assign a carer to a patient.' }
          end
        end

        def render_form
          form_with(
            model: [:admin, relationship],
            class: 'space-y-6'
          ) do |f|
            render_errors if relationship.errors.any?
            render_carer_field(f)
            render_patient_field(f)
            render_relationship_type_field(f)
            render_submit_button(f)
          end
        end

        def render_errors
          div(class: 'rounded-md bg-red-50 p-4') do
            div(class: 'flex') do
              div(class: 'ml-3') do
                h3(class: 'text-sm font-medium text-red-800') do
                  "#{relationship.errors.count} error(s) prohibited this relationship from being saved:"
                end
                div(class: 'mt-2 text-sm text-red-700') do
                  ul(class: 'list-disc space-y-1 pl-5') do
                    relationship.errors.full_messages.each do |message|
                      li { message }
                    end
                  end
                end
              end
            end
          end
        end

        def render_carer_field(form)
          div do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'carer_relationship_carer_id') { 'Carer' }
              form.select(
                :carer_id,
                carers.map { |p| [p.name, p.id] },
                { prompt: 'Select a carer...' },
                class: input_classes,
                id: 'carer_relationship_carer_id'
              )
            end
          end
        end

        def render_patient_field(form)
          div do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'carer_relationship_patient_id') { 'Patient' }
              form.select(
                :patient_id,
                patients.map { |p| [p.name, p.id] },
                { prompt: 'Select a patient...' },
                class: input_classes,
                id: 'carer_relationship_patient_id'
              )
            end
          end
        end

        def render_relationship_type_field(form)
          div do
            render RubyUI::FormField.new do
              render RubyUI::FormFieldLabel.new(for: 'carer_relationship_relationship_type') { 'Relationship type' }
              form.select(
                :relationship_type,
                RELATIONSHIP_TYPES,
                { prompt: 'Select relationship type...' },
                class: input_classes,
                id: 'carer_relationship_relationship_type'
              )
            end
          end
        end

        def render_submit_button(form)
          div(class: 'flex items-center gap-4') do
            form.submit(
              relationship.new_record? ? 'Create Relationship' : 'Update Relationship',
              class: 'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
                     'px-4 py-2 h-10 text-sm bg-primary text-primary-foreground hover:bg-primary/90'
            )
            a(
              href: '/admin/carer_relationships',
              class: 'text-sm text-slate-600 hover:text-slate-900'
            ) { 'Cancel' }
          end
        end

        def input_classes
          'w-full rounded-md border border-input bg-background px-3 py-2 text-sm ' \
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring'
        end
      end
    end
  end
end
