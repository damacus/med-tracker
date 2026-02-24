# frozen_string_literal: true

module Components
  module Locations
    class FormView < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::Pluralize

      attr_reader :location, :title, :subtitle

      def initialize(location:, title:, subtitle: nil)
        @location = location
        @title = title
        @subtitle = subtitle
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
            class: 'mx-auto w-16 h-16 rounded-[1.5rem] bg-primary/10 flex items-center justify-center ' \
                   'text-primary shadow-inner mb-6'
          ) do
            render Icons::Home.new(size: 32)
          end
          Heading(level: 1, size: '8', class: 'font-black tracking-tight text-slate-900') { title }
          Text(size: '3', class: 'text-slate-400') { subtitle } if subtitle
        end
      end

      def render_form
        form_with(
          model: location,
          class: 'space-y-8',
          data: { testid: 'location-form' }
        ) do |form|
          render_errors if location.errors.any?

          Card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-white') do
            div(class: 'p-10 space-y-8') do
              render_name_field(form)
              render_description_field(form)
            end

            div(class: 'px-10 py-6 bg-slate-50/50 border-t border-slate-100 flex items-center justify-between gap-4') do
              Link(href: locations_path, variant: :ghost, class: 'font-bold text-slate-400 hover:text-slate-600') do
                'Back to Locations'
              end
              Button(type: :submit, variant: :primary, size: :lg,
                     class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
                'Save Location'
              end
            end
          end
        end
      end

      def render_errors
        render RubyUI::Alert.new(variant: :destructive, class: 'mb-8 rounded-2xl border-none shadow-sm') do
          div(class: 'flex items-start gap-3') do
            render Icons::AlertCircle.new(size: 20)
            div do
              Heading(level: 2, size: '3', class: 'font-bold mb-1') do
                plain pluralize(location.errors.count, 'error')
              end
              ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1') do
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
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { 'Name' }
          render RubyUI::Input.new(
            type: :text,
            name: 'location[name]',
            id: 'location_name',
            value: location.name,
            required: true,
            placeholder: 'e.g. Home, School, Grandma\'s House',
            class: 'rounded-2xl border-slate-200 bg-white py-4 px-4 focus:ring-2 focus:ring-primary/10 ' \
                   "focus:border-primary transition-all #{field_error_class(location, :name)}"
          )
          render_field_error(location, :name)
        end
      end

      def render_description_field(_form)
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'location_description',
            class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1'
          ) { 'Description (optional)' }
          render RubyUI::Textarea.new(
            name: 'location[description]',
            id: 'location_description',
            rows: 3,
            placeholder: 'Describe this location...',
            class: 'rounded-2xl border-slate-200 bg-white p-4 focus:ring-2 focus:ring-primary/10 ' \
                   'focus:border-primary transition-all resize-none'
          ) { location.description }
        end
      end
    end
  end
end
