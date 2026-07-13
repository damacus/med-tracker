# frozen_string_literal: true

module Components
  module Medications
    class ListItemComponent < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medication, :inventory_query_params, :can_update, :can_refill, :can_destroy

      def initialize(medication:, inventory_query_params: {}, can_update: false, can_refill: false, can_destroy: false)
        @medication = medication
        @inventory_query_params = inventory_query_params
        @can_update = can_update
        @can_refill = can_refill
        @can_destroy = can_destroy
        super()
      end

      def view_template
        m3_card(
          id: tenant_dom_id(medication),
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-card ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex items-start gap-3 min-w-0') do
              render_medication_icon
              div(class: 'min-w-0 space-y-2') do
                m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight break-words leading-tight') do
                  medication.display_name
                end
                Badge(variant: :outlined, class: 'w-fit rounded-full text-[10px]') { medication.location.name }
              end
            end
          end

          CardContent(class: 'flex-grow space-y-6 px-8 pb-4') do
            if medication.description.present?
              m3_text(size: '2', class: 'text-on-surface-variant line-clamp-2 leading-relaxed') do
                medication.description
              end
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

      def render_supply_bar
        div(class: 'space-y-2') do
          div(
            class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                   "tracking-widest #{presenter.list_inventory_text_class}"
          ) do
            span { t('medications.index.inventory_level') }
            span { presenter.inventory_units_label }
          end
          render Components::Shared::SupplyMeter.new(
            percentage: presenter.supply_level.percentage,
            label: t('medications.index.inventory_level'),
            fill_class: presenter.list_inventory_text_class,
            track_class: 'h-1.5 bg-card',
            testid: 'medication-list-stock-meter'
          )
        end
      end

      def render_medication_icon
        render Components::Shared::MedicationIcon.new(
          medication: medication,
          size: 24,
          class: 'mt-1 shrink-0 text-on-surface-variant group-hover:text-primary'
        )
      end

      def render_actions
        div(class: 'flex items-center gap-2 w-full') do
          Link(
            href: medication_path(medication),
            variant: :outlined,
            size: :sm,
            class: 'flex-1 rounded-shape-full border-border bg-card ' \
                   'hover:bg-card text-on-surface-variant',
            data: { turbo_frame: '_top' }
          ) do
            t('medications.index.view')
          end
          if can_update
            Link(
              href: edit_medication_path(medication, return_to: medications_path(inventory_query_params)),
              variant: :outlined,
              size: :sm,
              class: 'rounded-shape-full w-11 h-11 p-0 border-border bg-card ' \
                     'hover:bg-card text-on-surface-variant',
              data: { turbo_frame: '_top' },
              aria_label: t('medications.index.edit', default: 'Edit medication')
            ) do
              render Icons::Pencil.new(size: 16, aria_hidden: 'true')
            end
          end
          if can_refill
            refill_classes = if medication.reorder_received?
                               'flex items-center justify-center rounded-shape-full w-11 h-11 p-0'
                             else
                               'flex items-center justify-center rounded-shape-full w-11 h-11 p-0 ' \
                                 'border-border bg-card ' \
                                 'hover:bg-card text-on-surface-variant'
                             end

            render Components::Medications::RefillModal.new(
              medication: medication,
              button_variant: medication.reorder_received? ? :primary : :outline,
              button_class: refill_classes,
              icon_only: true
            )
          end
          render_delete_dialog if can_destroy
        end
      end

      def render_delete_dialog
        AlertDialog do
          AlertDialogTrigger do
            m3_button(variant: :text, size: :sm,
                      class: 'rounded-shape-full w-11 h-11 p-0 text-on-surface-variant ' \
                             'hover:text-destructive hover:bg-destructive/5',
                      aria_label: t('medications.index.delete', default: 'Delete medication')) do
              render Icons::Trash.new(size: 18, aria_hidden: 'true')
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { t('medications.index.delete_dialog.title') }
              AlertDialogDescription do
                t('medications.index.delete_dialog.confirm', name: medication.display_name)
              end
            end
            AlertDialogFooter do
              AlertDialogCancel { t('medications.index.delete_dialog.cancel') }
              form_with(url: medication_path(medication), method: :delete, class: 'inline') do
                m3_button(variant: :destructive, type: :submit, class: 'shadow-elevation-2') do
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
