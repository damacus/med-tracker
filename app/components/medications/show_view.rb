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
        div(
          id: tenant_dom_target("medication_show_#{medication.id}"),
          class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12'
        ) do
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
          div(class: 'flex items-center gap-6 min-w-0') do
            div(
              class: 'w-20 h-20 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary shadow-inner flex-shrink-0',
              data: { testid: 'medication-hero-icon' }
            ) do
              render Icons::Inventory.new(size: 32)
            end
            div(class: 'space-y-1 min-w-0') do
              m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] opacity-40 block mb-1 font-black') do
                t('medications.show.profile')
              end
              m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight break-words') do
                medication.display_name
              end
              div(class: 'flex items-center gap-1 mt-1') do
                render Icons::Home.new(size: 14, class: 'text-on-surface-variant')
                m3_text(variant: :label_medium, class: 'text-on-surface-variant font-medium') do
                  medication.location.name
                end
              end
            end
          end

          div(class: 'flex gap-3') do
            if can_update?
              m3_link(
                href: edit_medication_path(medication, return_to: medication_path(medication)),
                variant: :outlined,
                size: :lg,
                class: 'bg-card shadow-sm hover:shadow-md transition-shadow'
              ) do
                render Icons::Pencil.new(size: 16, class: 'mr-2 text-primary')
                plain t('medications.show.edit_details')
              end
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

          render_reorder_actions if can_refill?
          render_refill_modal if can_refill?
          render_adjust_inventory_modal if can_update?
        end
      end

      def render_reorder_actions
        return render_order_form if medication.reorder_status.nil?

        render_order_details if medication.reorder_ordered?
      end

      def render_order_form
        form(
          action: mark_as_ordered_medication_path(medication),
          method: :post,
          class: 'col-span-2 space-y-3 rounded-shape-xl border border-border/60 bg-card p-4',
          data_turbo: 'false'
        ) do
          input(type: :hidden, name: :_method, value: :patch)
          input(type: :hidden, name: :authenticity_token, value: view_context.form_authenticity_token)
          render_order_field(:supplier, t('medications.show.order_supplier'), type: :text)
          render_order_field(:quantity, t('medications.show.order_quantity'), type: :number, step: '0.01', min: '0')
          render_order_field(:expected_arrival_on, t('medications.show.expected_arrival'), type: :date)
          button(
            type: :submit,
            class: 'w-full justify-center whitespace-nowrap inline-flex items-center rounded-shape-full ' \
                   'font-medium transition-all state-layer bg-primary text-on-primary shadow-elevation-1 ' \
                   'hover:shadow-elevation-2 h-12 px-6 text-base'
          ) do
            render Icons::Clock.new(size: 18, class: 'mr-2')
            span { t('medications.show.mark_as_ordered') }
          end
        end
      end

      def render_order_field(name, label, **input_options)
        field_id = "order_#{name}"
        div(class: 'space-y-1') do
          label(for: field_id, class: 'text-sm font-medium text-on-surface') { label }
          m3_input(
            id: field_id,
            name: "order[#{name}]",
            value: order_field_value(name),
            class: 'w-full',
            **input_options
          )
        end
      end

      def order_field_value(name)
        attribute = name == :expected_arrival_on ? name : :"order_#{name}"
        medication.public_send(attribute) if medication.respond_to?(attribute)
      end

      def render_order_details
        div(class: 'col-span-2 space-y-3 rounded-shape-xl border border-border/60 bg-card p-4') do
          m3_heading(variant: :title_small, level: 3) { t('medications.show.order_details') }
          render_order_detail(t('medications.show.order_supplier'), medication.order_supplier)
          render_order_detail(t('medications.show.order_quantity'), medication.order_quantity)
          render_order_detail(t('medications.show.expected_arrival'), formatted_expected_arrival)
          m3_link(
            href: mark_as_received_medication_path(medication),
            variant: :filled,
            size: :lg,
            class: 'w-full justify-center',
            data: { turbo_method: :patch }
          ) do
            render Icons::Check.new(size: 18, class: 'mr-2')
            span { t('medications.show.mark_as_received') }
          end
        end
      end

      def render_order_detail(label, value)
        return if value.blank?

        div(class: 'flex items-center justify-between gap-3 text-sm') do
          span(class: 'text-on-surface-variant') { label }
          span(class: 'font-semibold text-on-surface text-right') { value.to_s }
        end
      end

      def formatted_expected_arrival
        I18n.l(medication.expected_arrival_on, format: :long) if medication.expected_arrival_on
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

      def render_adjust_inventory_modal
        render Components::Medications::AdjustInventoryModal.new(
          medication: medication,
          button_variant: :filled,
          button_size: :lg,
          button_class: 'w-full justify-center col-span-2',
          button_label: t('medications.show.adjust_inventory')
        )
      end

      def render_dosages_section
        render Components::Medications::DoseHistoryComponent.new(medication: medication)
      end

      def can_update?
        view_context.policy(medication).update?
      rescue NoMethodError
        true
      end

      def can_refill?
        view_context.policy(medication).refill?
      rescue NoMethodError
        true
      end

      def header_content_offset_class
        'md:pl-[6.5rem]'
      end
    end
  end
end
