# frozen_string_literal: true

module Components
  module Medications
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :medications, :current_category, :categories, :locations, :current_location_id, :wizard_variant

      def initialize(medications:, current_category: nil, categories: [], locations: [], # rubocop:disable Metrics/ParameterLists
                     current_location_id: nil, wizard_variant: 'fullpage')
        @medications = medications
        @current_category = current_category
        @categories = categories
        @locations = locations
        @current_location_id = current_location_id
        @wizard_variant = wizard_variant
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'medications-list' }) do
          render_header
          render_filters_section
          render_medications_grid
          turbo_frame_tag 'modal'
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
            div(class: 'flex gap-3') do
              Link(
                href: add_medication_path,
                variant: :outline,
                size: :lg,
                class: 'rounded-2xl font-bold text-sm'
              ) do
                render Icons::PlusCircle.new(size: 20, class: 'mr-2')
                span { 'Add Schedule' }
              end
              Link(
                href: new_medication_path,
                variant: :primary,
                size: :lg,
                class: 'rounded-2xl font-bold text-sm shadow-lg shadow-primary/20',
                **wizard_link_data
              ) do
                render Icons::Pill.new(size: 20, class: 'mr-2')
                span { t('medications.index.add_medication') }
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
                can_manage: manageable_medication?(medication)
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

      def manageable_medication?(medication)
        view_context.policy(medication).update?
      end
    end
  end
end
