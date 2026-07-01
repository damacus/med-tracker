# frozen_string_literal: true

module Components
  module Admin
    module Households
      class EditView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize

        attr_reader :household

        def initialize(household:)
          @household = household
          super()
        end

        def view_template
          div(id: 'admin-household', class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-3xl space-y-8') do
            render_header
            render_form
          end
        end

        private

        def render_header
          div(class: 'space-y-2') do
            m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
              t('admin.households.title')
            end
            m3_text(weight: 'muted', class: 'block') { t('admin.households.subtitle') }
          end
        end

        def render_form
          form_with(url: admin_household_path, method: :patch, class: 'space-y-8') do
            m3_card(variant: :elevated, class: 'overflow-hidden border-none shadow-elevation-3 rounded-[2.5rem]') do
              div(class: 'p-10 space-y-6') do
                render_errors if household.errors.any?
                render_name_field
                render_actions
              end
            end
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'rounded-shape-xl border-none shadow-elevation-1') do
            div(class: 'space-y-1') do
              m3_heading(variant: :title_medium, level: 2, class: 'font-bold') do
                plain "#{pluralize(household.errors.count, 'error')} prevented this household from being saved:"
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                household.errors.full_messages.each { |message| li { message } }
              end
            end
          end
        end

        def render_name_field
          div(class: 'space-y-2') do
            render RubyUI::FormFieldLabel.new(
              for: 'household_name',
              class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
            ) do
              plain t('admin.households.name')
              span(class: 'text-error ml-0.5') { ' *' }
            end
            m3_input(
              type: :text,
              name: 'household[name]',
              id: 'household_name',
              value: household.name,
              required: true,
              class: [
                'rounded-shape-sm border-outline-variant bg-surface-container-lowest py-4 px-4',
                field_error_class(household, :name)
              ].join(' '),
              **field_error_attributes(household, :name, input_id: 'household_name')
            )
            render_field_error(household, :name, input_id: 'household_name')
          end
        end

        def render_actions
          div(class: 'flex items-center justify-between gap-4 pt-2') do
            m3_link(href: admin_root_path, variant: :text, size: :lg) { t('admin.users.form.cancel') }
            m3_button(
              type: :submit,
              variant: :filled,
              size: :lg,
              class: 'px-8 rounded-2xl shadow-lg shadow-primary/20'
            ) do
              t('admin.households.save')
            end
          end
        end
      end
    end
  end
end
