# frozen_string_literal: true

module Components
  module Medicines
    class ShowView < Components::Base
      attr_reader :medicine, :notice

      def initialize(medicine:, notice: nil)
        @medicine = medicine
        @notice = notice
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-4xl') do
          render_notice if notice.present?
          render_header
          render_details
          render_actions
        end
      end

      private

      def render_notice
        render RubyUI::Alert.new(variant: :success, class: 'mb-6') do
          plain(notice)
        end
      end

      def render_header
        div(class: 'mb-8') do
          p(class: 'text-sm font-medium uppercase tracking-wide text-slate-500 mb-2') { 'Medicine Profile' }
          h1(class: 'text-4xl font-bold text-slate-900') { medicine.name }
        end
      end

      def render_details
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 mb-8') do
          render_description_card
          render_dosage_card
          render_supply_card
          render_stock_card
          render_reorder_card
          render_warnings_card if medicine.warnings.present?
        end
      end

      def render_description_card
        Card(class: 'md:col-span-2') do
          CardHeader do
            CardTitle(class: 'text-lg') { 'Description' }
          end
          CardContent do
            p(class: 'text-slate-700') { medicine.description.presence || 'No description provided.' }
          end
        end
      end

      def render_dosage_card
        Card do
          CardHeader do
            CardTitle(class: 'text-lg') { 'Standard Dosage' }
          end
          CardContent do
            render_dosage_content
          end
        end
      end

      def render_dosage_content
        if dosage_specified?
          render_dosage_display
        else
          p(class: 'text-slate-600') { 'Not specified' }
        end
      end

      def dosage_specified?
        medicine.dosage_amount.present? && medicine.dosage_unit.present?
      end

      def render_dosage_display
        p(class: 'text-3xl font-bold text-slate-900') do
          plain medicine.dosage_amount.to_s
          span(class: 'text-lg font-medium text-slate-600 ml-2') { medicine.dosage_unit }
        end
      end

      def render_supply_card
        Card do
          CardHeader do
            CardTitle(class: 'text-lg') { 'Current Supply' }
          end
          CardContent do
            p(class: 'text-3xl font-bold text-slate-900') { (medicine.current_supply || 0).to_s }
          end
        end
      end

      def render_stock_card
        Card do
          CardHeader do
            CardTitle(class: 'text-lg') { 'Stock on Hand' }
          end
          CardContent do
            p(class: 'text-3xl font-bold text-slate-900') { (medicine.stock || 0).to_s }
            p(class: 'text-sm text-destructive font-semibold mt-2') { '⚠️ Low Stock' } if medicine.low_stock?
          end
        end
      end

      def render_reorder_card
        Card do
          CardHeader do
            CardTitle(class: 'text-lg') { 'Reorder Threshold' }
          end
          CardContent do
            p(class: 'text-3xl font-bold text-slate-900') { (medicine.reorder_threshold || 0).to_s }
          end
        end
      end

      def render_warnings_card
        Card(class: 'md:col-span-2 border-amber-200 bg-amber-50') do
          CardHeader do
            CardTitle(class: 'text-lg text-amber-700') { '⚠️ Warnings' }
          end
          CardContent do
            p(class: 'text-amber-800') { medicine.warnings }
          end
        end
      end

      def render_actions
        div(class: 'flex gap-3') do
          a(
            href: edit_medicine_path(medicine),
            class: button_primary_classes
          ) { 'Edit Medicine' }

          a(
            href: medicines_path,
            class: button_secondary_classes
          ) { 'Back to List' }
        end
      end

      def button_primary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-10 text-sm bg-primary text-primary-foreground shadow hover:bg-primary/90'
      end

      def button_secondary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-10 text-sm border border-input bg-background hover:bg-accent hover:text-accent-foreground'
      end
    end
  end
end
