# frozen_string_literal: true

module Components
  module Locations
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :location, :title, :subtitle, :return_to

      def initialize(location:, title:, subtitle: nil, return_to: nil)
        @location = location
        @title = title
        @subtitle = subtitle
        @return_to = return_to
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
        div(class: 'text-center mb-10 space-y-2') do
          div(
            class: 'mx-auto w-16 h-16 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                   'text-primary shadow-inner mb-6'
          ) do
            render Icons::Home.new(size: 32)
          end
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') { title }
          m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: location,
          class: 'space-y-8',
          data: { testid: 'location-form' }
        ) do |form|
          render_errors if location.errors.any?
          input(type: 'hidden', name: 'return_to', value: return_to) if return_to.present?

          m3_card(variant: :elevated, class: 'overflow-hidden border-none shadow-elevation-3 rounded-[2.5rem]') do
            div(class: 'p-10 space-y-8') do
              render_name_field(form)
              render_description_field(form)
            end

            div(
              class: 'px-10 py-6 bg-surface-container-low border-t border-outline-variant/30 ' \
                     'flex items-center justify-between gap-4 rounded-b-[2.5rem]'
            ) do
              m3_link(href: return_to.presence || locations_path, variant: :text, size: :lg,
                      class: 'font-bold text-on-surface-variant hover:text-foreground transition-all') do
                t('people.show.back')
              end
              m3_button(type: :submit, variant: :filled, size: :lg,
                        class: 'px-8 rounded-shape-xl shadow-lg shadow-primary/20 transition-all') do
                t('locations.form.save')
              end
            end
          end
        end
      end

      def render_errors
        render RubyUI::Alert.new(variant: :destructive,
                                 class: 'mb-8 rounded-shape-xl border-none shadow-elevation-1') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              m3_heading(variant: :title_medium, level: 2, class: 'font-bold mb-1') do
                plain pluralize(location.errors.count, 'error')
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1 font-medium') do
                location.errors.full_messages.each do |message|
                  li { message }
                end
              end
            end
          end
        end
      end

      def render_name_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'location_name',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('people.form.name') }
          m3_input(
            type: :text,
            name: 'location[name]',
            id: 'location_name',
            value: location.name,
            required: true,
            placeholder: t('forms.locations.name_placeholder'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest ' \
                   'py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all ' \
                   "#{field_error_class(location, :name)}",
            **field_error_attributes(location, :name, input_id: 'location_name')
          )
          render_field_error(location, :name, input_id: 'location_name')
        end
      end

      def render_description_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'location_description',
            class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1'
          ) { t('locations.form.description_optional') }
          render RubyUI::Textarea.new(
            name: 'location[description]',
            id: 'location_description',
            rows: 3,
            placeholder: t('forms.locations.description_placeholder'),
            class: 'rounded-md border-outline-variant bg-surface-container-lowest ' \
                   'p-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          ) { location.description }
        end
      end
    end
  end
end
