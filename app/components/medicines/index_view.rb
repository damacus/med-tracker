# frozen_string_literal: true

module Components
  module Medicines
    class IndexView < Components::Base
      attr_reader :medicines

      def initialize(medicines:)
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8', data: { testid: 'medicines-list' }) do
          render_header
          render_medicines_grid
        end
      end

      private

      def render_header
        div(class: 'flex justify-between items-center mb-8') do
          h1(class: 'text-4xl font-bold text-slate-900') { 'Medicines' }
          a(
            href: new_medicine_path,
            class: button_primary_classes,
            data: { turbo_stream: true }
          ) { 'Add Medicine' }
        end
      end

      def render_medicines_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6', id: 'medicines') do
          medicines.each do |medicine|
            render_medicine_card(medicine)
          end
        end
      end

      def render_medicine_card(medicine)
        Card(id: "medicine_#{medicine.id}", class: 'h-full flex flex-col') do
          CardHeader do
            render_medicine_icon
            CardTitle(class: 'text-xl') { medicine.name }
          end

          CardContent(class: 'flex-grow space-y-2') do
            CardDescription { medicine.description } if medicine.description.present?
            render_medicine_details(medicine) if medicine.description.present?
          end

          CardFooter do
            render_medicine_actions(medicine)
          end
        end
      end

      def render_medicine_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700 mb-2') do
          svg(
            xmlns: 'http://www.w3.org/2000/svg',
            width: '20',
            height: '20',
            viewBox: '0 0 24 24',
            fill: 'none',
            stroke: 'currentColor',
            stroke_width: '2',
            stroke_linecap: 'round',
            stroke_linejoin: 'round'
          ) do |s|
            s.path(d: 'M10.5 20.5 10 21a2 2 0 0 1-2.828 0L4.343 18.172a2 2 0 0 1 0-2.828l.5-.5')
            s.path(d: 'm7 17-5-5')
            s.path(d: 'M13.5 3.5 14 3a2 2 0 0 1 2.828 0l2.829 2.828a2 2 0 0 1 0 2.829l-.5.5')
            s.path(d: 'm17 7 5 5')
            s.circle(cx: '12', cy: '12', r: '2')
          end
        end
      end

      def render_medicine_details(medicine)
        div(class: 'space-y-1 text-sm text-muted-foreground') do
          render_detail_row('Current Supply', medicine.current_supply)
          render_detail_row('Stock', medicine.stock)
          render_detail_row('Reorder Threshold', medicine.reorder_threshold)
          p(class: 'text-destructive font-semibold') { '⚠️ Low Stock' } if medicine.low_stock?
        end
      end

      def render_detail_row(label, value)
        p do
          strong { "#{label}: " }
          plain value.to_s
        end
      end

      def render_medicine_actions(medicine)
        div(class: 'flex h-5 items-center space-x-4 text-sm') do
          a(href: medicine_path(medicine), class: 'text-primary hover:underline') { 'View' }
          Separator(orientation: :vertical)
          a(href: edit_medicine_path(medicine), class: 'text-primary hover:underline') { 'Edit' }
          Separator(orientation: :vertical)
          a(
            href: medicine_path(medicine),
            class: 'text-destructive hover:underline',
            data: { turbo_method: :delete, turbo_confirm: 'Are you sure?' }
          ) { 'Delete' }
        end
      end

      def button_primary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-9 text-sm bg-primary text-primary-foreground shadow hover:bg-primary/90'
      end
    end
  end
end
