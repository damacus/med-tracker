# frozen_string_literal: true

module Components
  module People
    module CarerRelationships
      class FormView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Components::FormHelpers

        attr_reader :relationship, :patient, :carers, :current_user, :email

        def initialize(relationship:, patient:, carers:, current_user:, email: nil)
          @relationship = relationship
          @patient = patient
          @carers = carers
          @current_user = current_user
          @email = email
          super()
        end

        def view_template
          div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
            render_header
            render_form
          end
        end

        private

        def render_header
          div(class: 'mb-8 space-y-2 text-center md:text-left') do
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') do
              t('people.carer_relationships.new_title')
            end
            m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
              t('people.carer_relationships.new_subtitle', name: patient.name)
            end
          end
        end

        def render_form
          form_with(url: person_carer_relationships_path(patient), method: :post, class: 'space-y-6') do
            render_errors if relationship.errors.any?
            if current_user.administrator?
              render_admin_fields
            else
              render_parent_fields
            end
            render_actions
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
            ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
              relationship.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end

        def render_admin_fields
          render_carer_select
          render_relationship_type_select
        end

        def render_carer_select
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'carer_relationship_carer_id') do
              t('people.carer_relationships.carer')
            end
            select(name: 'carer_relationship[carer_id]', id: 'carer_relationship_carer_id', class: select_classes) do
              option(value: '') { t('people.carer_relationships.select_carer') }
              carers.each do |carer|
                option(value: carer.id) { carer.name }
              end
            end
          end
        end

        def render_relationship_type_select
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'carer_relationship_relationship_type') do
              t('people.carer_relationships.relationship_type')
            end
            select(
              name: 'carer_relationship[relationship_type]',
              id: 'carer_relationship_relationship_type',
              class: select_classes
            ) do
              CarerRelationship::RELATIONSHIP_TYPES.each do |label, value|
                option(value: value) { label }
              end
            end
          end
        end

        def render_parent_fields
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(for: 'carer_relationship_email') do
              t('people.carer_relationships.email')
            end
            m3_input(
              type: :email,
              name: 'carer_relationship[email]',
              id: 'carer_relationship_email',
              value: email,
              required: true,
              placeholder: t('people.carer_relationships.email_placeholder')
            )
          end
        end

        def render_actions
          div(class: 'flex gap-3 justify-end pt-4') do
            m3_link(href: person_path(patient), variant: :text) { t('people.carer_relationships.cancel') }
            m3_button(type: :submit, variant: :filled) { t('people.carer_relationships.submit') }
          end
        end
      end
    end
  end
end
