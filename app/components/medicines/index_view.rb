# frozen_string_literal: true

module Components
  module Medicines
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medicines

      def initialize(medicines:)
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8', data: { testid: 'medicines-list' }) do
          render_header
          render_medicines_grid
        end
      end

      private

      def render_header
        div(class: 'flex justify-between items-center mb-8') do
          Heading(level: 1) { 'Medicines' }
          Link(
            href: new_medicine_path,
            variant: :primary,
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
            Heading(level: 2, size: '5', class: 'font-semibold leading-none tracking-tight') { medicine.name }
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
          render Icons::Pill.new(size: 20)
        end
      end

      def render_medicine_details(medicine)
        div(class: 'space-y-1 text-sm text-muted-foreground') do
          render_detail_row('Current Supply', medicine.current_supply)
          render_detail_row('Stock', medicine.stock)
          render_detail_row('Reorder Threshold', medicine.reorder_threshold)
          Text(weight: 'semibold', class: 'text-destructive') { '⚠️ Low Stock' } if medicine.low_stock?
        end
      end

      def render_detail_row(label, value)
        Text do
          strong { "#{label}: " }
          plain value.to_s
        end
      end

      def render_medicine_actions(medicine)
        div(class: 'flex h-5 items-center space-x-4 text-sm') do
          Link(href: medicine_path(medicine), variant: :ghost, size: :sm) { 'View' }
          Separator(orientation: :vertical)
          Link(href: edit_medicine_path(medicine), variant: :ghost, size: :sm) { 'Edit' }
          Separator(orientation: :vertical)
          render_delete_dialog(medicine)
        end
      end

      def render_delete_dialog(medicine)
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive_outline, size: :sm) { 'Delete' }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Medicine' }
              AlertDialogDescription do
                "Are you sure you want to delete #{medicine.name}? This action cannot be undone."
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { 'Cancel' }
              form_with(url: medicine_path(medicine), method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit) { 'Delete' }
              end
            end
          end
        end
      end
    end
  end
end
