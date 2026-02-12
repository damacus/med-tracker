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
            Heading(level: 1) do
              relationship.new_record? ? 'New Carer Relationship' : 'Edit Carer Relationship'
            end
            Text(weight: 'muted', class: 'mt-2') { 'Assign a carer to a patient.' }
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
          render Components::Shared::ErrorSummary.new(model: relationship, resource_name: 'relationship')
        end

        def render_carer_field(_form)
          div do
            FormField do
              FormFieldLabel(for: 'carer_relationship_carer_id') { 'Carer' }
              select(
                name: 'carer_relationship[carer_id]',
                id: 'carer_relationship_carer_id',
                class: select_classes
              ) do
                option(value: '', selected: relationship.carer_id.blank?) { 'Select a carer...' }
                carers.each do |carer|
                  option(value: carer.id, selected: relationship.carer_id == carer.id) { carer.name }
                end
              end
            end
          end
        end

        def render_patient_field(_form)
          div do
            FormField do
              FormFieldLabel(for: 'carer_relationship_patient_id') { 'Patient' }
              select(
                name: 'carer_relationship[patient_id]',
                id: 'carer_relationship_patient_id',
                class: select_classes
              ) do
                option(value: '', selected: relationship.patient_id.blank?) { 'Select a patient...' }
                patients.each do |patient|
                  option(value: patient.id, selected: relationship.patient_id == patient.id) { patient.name }
                end
              end
            end
          end
        end

        def render_relationship_type_field(_form)
          div do
            FormField do
              FormFieldLabel(for: 'carer_relationship_relationship_type') { 'Relationship type' }
              select(
                name: 'carer_relationship[relationship_type]',
                id: 'carer_relationship_relationship_type',
                class: select_classes
              ) do
                option(value: '', selected: relationship.relationship_type.blank?) { 'Select relationship type...' }
                RELATIONSHIP_TYPES.each do |label, value|
                  option(value: value, selected: relationship.relationship_type == value) { label }
                end
              end
            end
          end
        end

        def render_submit_button(_form)
          div(class: 'flex items-center gap-4') do
            Button(type: :submit, variant: :primary) do
              relationship.new_record? ? 'Create Relationship' : 'Update Relationship'
            end
            Link(href: '/admin/carer_relationships', variant: :link) { 'Cancel' }
          end
        end
      end
    end
  end
end
