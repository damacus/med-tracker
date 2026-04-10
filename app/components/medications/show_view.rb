# frozen_string_literal: true

module Components
  module Medications
    class ShowView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :medication, :notice

      def initialize(medication:, notice: nil)
        @medication = medication
        @notice = notice
        super()
      end

      def view_template
        div(id: "medication_show_#{medication.id}", class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_description_section
              render_warnings_section if medication.warnings.present?
              render_dosages_section
            end

            div(class: 'space-y-8') do
              render Components::Medications::SupplyStatusCard.new(medication: medication)
              render_dosage_card
              render_actions_card
            end
          end
        end
      end

      private

      def render_notice
        render RubyUI::Alert.new(variant: :success, class: 'mb-8 rounded-2xl border-none shadow-sm') do
          plain(notice)
        end
      end

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-border') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-20 h-20 rounded-[2rem] bg-primary/10 flex items-center justify-center text-primary shadow-inner'
            ) do
              render Icons::Pill.new(size: 32)
            end
            div(class: 'space-y-1') do
              Text(size: '2', weight: 'black', class: 'uppercase tracking-[0.2em] font-bold opacity-40 block mb-1') do
                t('medications.show.profile')
              end
              Heading(level: 1, size: '8', class: 'font-black tracking-tight') { medication.name }
              div(class: 'flex items-center gap-1 mt-1') do
                render Icons::Home.new(size: 14, class: 'text-muted-foreground')
                Text(size: '2', class: 'text-muted-foreground') { medication.location.name }
              end
            end
          end

          div(class: 'flex gap-3') do
            Link(
              href: edit_medication_path(medication, return_to: medication_path(medication)),
              variant: :outline,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm bg-surface-container-lowest'
            ) do
              render Icons::Pencil.new(size: 16, class: 'mr-2')
              plain t('medications.show.edit_details')
            end
            Link(
              href: medications_path,
              variant: :ghost,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm text-muted-foreground hover:text-foreground'
            ) do
              t('medications.show.inventory')
            end
          end
        end
      end

      def render_description_section
        div(class: 'space-y-4') do
          Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { t('medications.show.overview') }
          Card(class: 'p-8') do
            Text(size: '3', class: 'text-muted-foreground leading-relaxed') do
              medication.description.presence || t('medications.show.no_description')
            end
          end
        end
      end

      def render_warnings_section
        render Components::Medications::WarningsComponent.new(medication: medication)
      end

      def render_dosage_card
        render Components::Medications::StandardDosageComponent.new(medication: medication)
      end

      def render_actions_card
        div(class: 'space-y-3') do
          Link(
            href: add_medication_path(medication_id: medication.id),
            variant: :outline,
            size: :lg,
            class: 'w-full py-6 rounded-2xl bg-surface-container-lowest border-border shadow-sm ' \
                   'flex items-center justify-center'
          ) do
            render Icons::PlusCircle.new(size: 18, class: 'mr-2 text-primary')
            span(class: 'font-semibold text-foreground') { t('medications.show.add_schedule') }
          end

          render Button.new(
            variant: :primary,
            class: 'w-full py-6 rounded-2xl shadow-lg shadow-primary/20 flex items-center justify-center'
          ) do
            render Icons::Activity.new(size: 18, class: 'mr-2')
            span(class: 'font-semibold') { t('medications.show.log_administration') }
          end

          # Reorder & Refill Actions Group
          div(class: 'grid grid-cols-2 gap-3') do
            render_reorder_actions
            render_refill_modal
          end
        end
      end

      def render_reorder_actions
        path, label, icon = if medication.reorder_status.nil?
                              [mark_as_ordered_medication_path(medication), t('medications.show.mark_as_ordered'), Icons::Clock]
                            elsif medication.reorder_ordered?
                              [mark_as_received_medication_path(medication), t('medications.show.mark_as_received'), Icons::Check]
                            end

        return unless path

        Link(
          href: path,
          variant: :outline,
          size: :lg,
          data: { turbo_method: :patch },
          class: 'w-full py-6 rounded-2xl bg-surface-container-lowest border-border shadow-sm ' \
                 'flex items-center justify-center'
        ) do
          render icon.new(size: 18, class: 'mr-2 text-primary')
          span(class: 'font-semibold text-foreground') { label }
        end
      end

      def render_refill_modal
        is_received = medication.reorder_received?
        base_classes = 'w-full py-6 rounded-2xl shadow-sm flex items-center justify-center'
        button_class = if is_received
                         base_classes
                       else
                         "#{base_classes} bg-surface-container-lowest border-border"
                       end

        render Components::Medications::RefillModal.new(
          medication: medication,
          button_variant: is_received ? :primary : :outline,
          button_class: button_class,
          button_label: is_received ? t('medications.show.complete_refill') : t('medications.show.refill_inventory')
        )
      end

      def render_dosages_section
        render Components::Medications::DoseHistoryComponent.new(medication: medication)
      end
    end
  end
end
