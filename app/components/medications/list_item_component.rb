# frozen_string_literal: true

module Components
  module Medications
    class ListItemComponent < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medication, :inventory_query_params, :can_manage

      def initialize(medication:, inventory_query_params: {}, can_manage: false)
        @medication = medication
        @inventory_query_params = inventory_query_params
        @can_manage = can_manage
        super()
      end

      def view_template
        Card(
          id: "medication_#{medication.id}",
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-cardest ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medication_icon
              render_status_badge
            end
            div(class: 'space-y-2') do
              Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { medication.name }
              Badge(variant: :outline, class: 'w-fit rounded-full text-[10px]') { medication.location.name }
            end
          end

          CardContent(class: 'flex-grow space-y-6 px-8 pb-4') do
            if medication.description.present?
              Text(size: '2', class: 'text-muted-foreground line-clamp-2 leading-relaxed') { medication.description }
            end

            div(class: 'pt-4 border-t border-border space-y-4') do
              render_supply_bar
            end
          end

          CardFooter(class: 'px-8 pb-8 pt-2 mt-auto') do
            render_actions
          end
        end
      end

      private

      def presenter
        @presenter ||= ::Medications::SupplyStatusPresenter.new(medication: medication)
      end

      def render_status_badge
        Badge(variant: presenter.status_variant) { presenter.status_label }
      end

      def render_supply_bar
        div(class: 'space-y-2') do
          div(
            class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                   'tracking-widest text-muted-foreground'
          ) do
            span { t('medications.index.inventory_level') }
            span { presenter.inventory_units_label }
          end
          div(class: 'h-1.5 w-full bg-card rounded-full overflow-hidden') do
            div(class: "h-full #{presenter.list_supply_bar_class} rounded-full transition-all duration-1000",
                style: "width: #{presenter.supply_level.percentage}%")
          end
        end
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-shape-xl bg-card flex items-center ' \
                 'justify-center text-muted-foreground ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_actions
        div(class: 'flex items-center gap-2 w-full') do
          Link(
            href: medication_path(medication),
            variant: :outline,
            size: :sm,
            class: 'flex-1 rounded-xl py-5 border-border bg-cardest ' \
                   'hover:bg-card text-muted-foreground'
          ) do
            t('medications.index.view')
          end
          Link(
            href: edit_medication_path(medication, return_to: medications_path(inventory_query_params)),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-border bg-cardest ' \
                   'hover:bg-card text-muted-foreground',
            aria_label: t('medications.index.edit', default: 'Edit medication')
          ) do
            render Icons::Pencil.new(size: 16)
          end
          if can_manage
            refill_classes = if medication.reorder_received?
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0'
                             else
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0 ' \
                                 'border-border bg-cardest ' \
                                 'hover:bg-card text-muted-foreground'
                             end

            render Components::Medications::RefillModal.new(
              medication: medication,
              button_variant: medication.reorder_received? ? :primary : :outline,
              button_class: refill_classes,
              icon_only: true
            )
          end
          render_delete_dialog
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :ghost, size: :sm,
                   class: 'rounded-xl w-10 h-10 p-0 text-muted-foreground ' \
                          'hover:text-destructive hover:bg-destructive/5',
                   aria_label: t('medications.index.delete', default: 'Delete medication')) do
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
