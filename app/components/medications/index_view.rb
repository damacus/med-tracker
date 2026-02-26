# frozen_string_literal: true

module Components
  module Medications
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medications

      def initialize(medications:)
        @medications = medications
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'medications-list' }) do
          render_header
          render_categories_section
          render_medications_grid
        end
      end

      private

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
          div do
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              t('medications.index.your_inventory')
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { t('medications.index.title') }
          end
          if view_context.policy(Medication).create?
            Link(
              href: new_medication_path,
              variant: :primary,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm shadow-lg shadow-primary/20',
              data: { turbo_stream: true }
            ) do
              render Icons::Pill.new(size: 20, class: 'mr-2')
              span { t('medications.index.add_medication') }
            end
          end
        end
      end

      def render_categories_section
        categories = medications.filter_map(&:category).uniq.sort
        return if categories.empty?

        div(class: 'mb-12 overflow-x-auto no-scrollbar') do
          div(class: 'flex gap-3 min-w-max pb-2') do
            Button(
              variant: :primary,
              class: 'rounded-full px-6 py-2 h-auto text-xs font-bold uppercase tracking-wider ' \
                     'shadow-md shadow-primary/10'
            ) { t('medications.index.all') }

            categories.each do |cat|
              Button(
                variant: :outline,
                class: 'rounded-full px-6 py-2 h-auto text-xs font-bold uppercase tracking-wider'
              ) { cat.pluralize.titleize }
            end
          end
        end
      end

      def render_medications_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: 'medications') do
          medications.each do |medication|
            render_medication_card(medication)
          end
        end
      end

      def render_medication_card(medication)
        Card(
          id: "medication_#{medication.id}",
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-white ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medication_icon
              status_badge(medication)
            end
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { medication.name }
          end

          CardContent(class: 'flex-grow space-y-6 px-8 pb-4') do
            if medication.description.present?
              Text(size: '2', class: 'text-slate-400 line-clamp-2 leading-relaxed') { medication.description }
            end

            div(class: 'pt-4 border-t border-slate-50 space-y-4') do
              render_supply_bar(medication)
            end
          end

          CardFooter(class: 'px-8 pb-8 pt-2 mt-auto') do
            render_medication_actions(medication)
          end
        end
      end

      def status_badge(medication)
        if medication.reorder_ordered?
          Badge(variant: :default) { t('medications.reorder_statuses.ordered') }
        elsif medication.reorder_received?
          Badge(variant: :success) { t('medications.reorder_statuses.received') }
        elsif medication.out_of_stock?
          Badge(variant: :destructive) { t('dashboard.statuses.out_of_stock') }
        elsif medication.low_stock?
          Badge(variant: :warning) { t('medications.show.low_stock_alert') }
        else
          Badge(variant: :success) { t('medications.index.in_stock', default: 'In Stock') }
        end
      end

      def render_supply_bar(medication)
        percentage = medication.supply_percentage
        bar_color = medication.low_stock? ? 'bg-destructive' : 'bg-primary'

        div(class: 'space-y-2') do
          div(
            class: 'flex justify-between items-center text-[10px] font-black uppercase tracking-widest text-slate-400'
          ) do
            span { t('medications.index.inventory_level') }
            span { "#{medication.current_supply} #{t('medications.index.units')}" }
          end
          div(class: 'h-1.5 w-full bg-slate-50 rounded-full overflow-hidden') do
            div(class: "h-full #{bar_color} rounded-full transition-all duration-1000", style: "width: #{percentage}%")
          end
        end
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_medication_actions(medication)
        div(class: 'flex items-center gap-2 w-full') do
          Link(
            href: medication_path(medication),
            variant: :outline,
            size: :sm,
            class: 'flex-1 rounded-xl py-5 border-slate-100 bg-white hover:bg-slate-50 text-slate-600'
          ) do
            t('medications.index.view')
          end
          Link(
            href: edit_medication_path(medication, return_to: medications_path),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-slate-100 bg-white hover:bg-slate-50 text-slate-400'
          ) do
            render Icons::Pencil.new(size: 16)
          end
          if view_context.policy(medication).update?
            refill_classes = if medication.reorder_received?
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0'
                             else
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0 ' \
                                 'border-slate-100 bg-white hover:bg-slate-50 text-slate-400'
                             end

            render Components::Medications::RefillModal.new(
              medication: medication,
              button_variant: medication.reorder_received? ? :primary : :outline,
              button_class: refill_classes,
              icon_only: true
            )
          end
          render_delete_dialog(medication)
        end
      end

      def render_delete_dialog(medication)
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :ghost, size: :sm,
                   class: 'rounded-xl w-10 h-10 p-0 text-slate-300 hover:text-destructive hover:bg-destructive/5') do
              render Icons::Trash.new(size: 18)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('medications.index.delete_dialog.title') }
              AlertDialogDescription do
                t('medications.index.delete_dialog.confirm', name: medication.name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { t('medications.index.delete_dialog.cancel') }
              form_with(url: medication_path(medication), method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  t('medications.index.delete_dialog.submit')
                end
              end
            end
          end
        end
      end
    end
  end
end
