# frozen_string_literal: true

module Components
  module Medications
    class ShowView < Components::Base
      attr_reader :medication, :notice

      def initialize(medication:, notice: nil, nhs_guidance: nil)
        @medication = medication
        @notice = notice
        @nhs_guidance = nhs_guidance
        super()
      end

      def view_template
        div(id: "medication_show_#{medication.id}", class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_notice if notice.present?
          render_header

          div(class: "grid grid-cols-1 lg:grid-cols-3 gap-12 #{header_content_offset_class}",
              data: { testid: 'medication-content' }) do
            div(class: 'lg:col-span-2 space-y-8') do
              render_description_section
              render_nhs_guidance_frame
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
        render RubyUI::Alert.new(variant: :success, class: 'mb-8 rounded-shape-xl border-none shadow-sm') do
          plain(notice)
        end
      end

      def render_header
        div(class: 'flex flex-col justify-between gap-6 border-b border-border/60 pb-8 md:flex-row md:items-end') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-20 h-20 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary shadow-inner',
              data: { testid: 'medication-hero-icon' }
            ) do
              render Icons::Pill.new(size: 32)
            end
            div(class: 'space-y-1') do
              m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] opacity-40 block mb-1 font-black') do
                t('medications.show.profile')
              end
              m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') { medication.name }
              div(class: 'flex items-center gap-1 mt-1') do
                render Icons::Home.new(size: 14, class: 'text-on-surface-variant')
                m3_text(variant: :label_medium, class: 'text-on-surface-variant font-medium') do
                  medication.location.name
                end
              end
            end
          end

          div(class: 'flex gap-3') do
            m3_link(
              href: edit_medication_path(medication, return_to: medication_path(medication)),
              variant: :outlined,
              size: :lg,
              class: 'bg-card shadow-sm hover:shadow-md transition-shadow'
            ) do
              render Icons::Pencil.new(size: 16, class: 'mr-2 text-primary')
              plain t('medications.show.edit_details')
            end
            m3_link(
              href: medications_path,
              variant: :text,
              size: :lg,
              class: 'text-on-surface-variant hover:text-foreground font-bold'
            ) do
              t('medications.show.inventory')
            end
          end
        end
      end

      def render_description_section
        div(class: 'space-y-4') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('medications.show.overview')
          end
          m3_card(variant: :elevated, class: 'border border-border/60 p-8') do
            m3_text(variant: :body_large, class: 'text-on-surface-variant leading-relaxed font-medium') do
              medication.description.presence || t('medications.show.no_description')
            end
          end
        end
      end

      def render_warnings_section
        render Components::Medications::WarningsComponent.new(medication: medication)
      end

      def render_nhs_guidance_frame
        render Components::Medications::NhsGuidanceFrame.new(
          medication: medication,
          guidance: @nhs_guidance,
          src: @nhs_guidance.present? ? nil : nhs_guidance_medication_path(medication)
        )
      end

      def render_dosage_card
        render Components::Medications::StandardDosageComponent.new(medication: medication)
      end

      def render_actions_card
        div(class: 'grid grid-cols-2 gap-3') do
          m3_link(
            href: add_medication_path(medication_id: medication.id),
            variant: :filled,
            size: :lg,
            class: 'w-full justify-center'
          ) do
            render Icons::PlusCircle.new(size: 18, class: 'mr-2')
            span { t('medications.show.add_schedule') }
          end

          m3_link(
            href: administration_medication_path(medication),
            variant: :filled,
            size: :lg,
            class: 'w-full justify-center',
            data: { turbo_frame: 'modal' }
          ) do
            render Icons::Activity.new(size: 18, class: 'mr-2')
            span { t('medications.show.log_administration') }
          end

          render_reorder_actions
          render_refill_modal
        end
      end

      def render_reorder_actions
        config = if medication.reorder_status.nil?
                   { path: mark_as_ordered_medication_path(medication), label: t('medications.show.mark_as_ordered'),
                     icon: Icons::Clock }
                 elsif medication.reorder_ordered?
                   { path: mark_as_received_medication_path(medication), label: t('medications.show.mark_as_received'),
                     icon: Icons::Check }
                 end

        return unless config

        m3_link(
          href: config[:path],
          variant: :filled,
          size: :lg,
          class: 'w-full justify-center',
          data: { turbo_method: :patch }
        ) do
          render config[:icon].new(size: 18, class: 'mr-2')
          span { config[:label] }
        end
      end

      def render_refill_modal
        render Components::Medications::RefillModal.new(
          medication: medication,
          button_variant: :filled,
          button_size: :lg,
          button_class: 'w-full justify-center',
          button_label: t('medications.show.refill_inventory'),
          icon: Icons::RefreshCw
        )
      end

      def render_dosages_section
        render Components::Medications::DoseHistoryComponent.new(medication: medication)
      end

      def header_content_offset_class
        'md:pl-[6.5rem]'
      end
    end
  end
end
