# frozen_string_literal: true

module Components
  module Medications
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :medications, :current_category, :categories, :locations, :current_location_id, :wizard_variant,
                  :frame_only

      def initialize(medications:, current_category: nil, categories: [], locations: [], # rubocop:disable Metrics/ParameterLists
                     current_location_id: nil, wizard_variant: 'fullpage', frame_only: false)
        @medications = medications
        @current_category = current_category
        @categories = categories
        @locations = locations
        @current_location_id = current_location_id
        @wizard_variant = wizard_variant
        @frame_only = frame_only
        super()
      end

      def view_template
        return render_inventory_frame if frame_only

        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'medications-list' }) do
          render_header
          render_inventory_frame
          turbo_frame_tag 'modal'
        end
      end

      private

      def render_inventory_frame
        turbo_frame_tag 'medications_inventory',
                        class: header_content_offset_class,
                        data: { testid: 'medications-content' } do
          render_filters_section
          render_medications_grid
        end
      end

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-border mb-12') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-20 h-20 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary shadow-inner'
            ) do
              render Icons::Inventory.new(size: 32)
            end
            div(class: 'space-y-1') do
              m3_text(size: '2', weight: 'bold',
                      class: 'uppercase tracking-[0.2em] opacity-40 block mb-1 font-black') do
                t('medications.index.your_inventory')
              end
              m3_heading(level: 1, size: '8', class: 'font-black tracking-tight') { t('medications.index.title') }
            end
          end

          if can_create_medication? || can_refill_medication?
            div(
              class: 'medications-index-actions flex w-full flex-wrap gap-3 md:w-auto md:flex-nowrap md:justify-end'
            ) do
              render Components::Medications::InventoryScanModal.new if can_refill_medication?
              if can_create_medication?
                m3_link(
                  href: add_medication_path(return_to: medications_path),
                  variant: :outlined,
                  size: :lg,
                  class: 'max-w-full justify-center rounded-shape-full font-bold text-sm bg-card shadow-sm ' \
                         'border-border',
                  data: { turbo_frame: 'modal' }
                ) do
                  render Icons::PlusCircle.new(size: 20, class: 'mr-2 text-primary')
                  span { 'Add Schedule' }
                end
                m3_link(
                  href: new_medication_path,
                  variant: :filled,
                  size: :lg,
                  class: 'max-w-full justify-center rounded-shape-full font-bold text-sm shadow-elevation-2',
                  **wizard_link_data
                ) do
                  render Icons::Medication.new(size: 20, class: 'mr-2')
                  span { t('medications.index.add_medication') }
                end
              end
            end
          end
        end
      end

      def render_filters_section
        render Components::Medications::SearchComponent.new(
          current_category: current_category,
          categories: categories,
          locations: locations,
          current_location_id: current_location_id
        )
      end

      def inventory_query_params
        params = {}
        params[:category] = current_category if current_category.present?
        params[:location_id] = current_location_id if current_location_id.present?
        params
      end

      def wizard_link_data
        %w[modal slideover].include?(wizard_variant) ? { data: { turbo_frame: 'modal' } } : {}
      end

      def render_medications_grid
        if medications.any?
          div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: 'medications') do
            medications.each do |medication|
              render Components::Medications::ListItemComponent.new(
                medication: medication,
                inventory_query_params: inventory_query_params,
                can_update: updatable_medication?(medication),
                can_refill: refillable_medication?(medication),
                can_destroy: destroyable_medication?(medication)
              )
            end
          end
        else
          render Components::Shared::EmptyState.new(
            title: t('medications.index.empty_title'),
            description: t('medications.index.empty_description')
          )
        end
      end

      def can_create_medication?
        view_context.policy(Medication).create?
      end

      def can_refill_medication?
        view_context.policy(Medication).refill?
      end

      def updatable_medication?(medication)
        view_context.policy(medication).update?
      end

      def refillable_medication?(medication)
        view_context.policy(medication).refill?
      end

      def destroyable_medication?(medication)
        view_context.policy(medication).destroy?
      end

      def header_content_offset_class
        'md:pl-[6.5rem]'
      end
    end
  end
end
