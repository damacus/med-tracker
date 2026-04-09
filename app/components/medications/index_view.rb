# frozen_string_literal: true

module Components
  module Medications
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::FormWith
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
        return if locations.empty? && categories.empty?

        div(class: 'mb-12 grid grid-cols-1 md:grid-cols-2 gap-6 max-w-3xl') do
          form_with(url: medications_path, method: :get, class: 'contents', data: { controller: 'form-submit' }) do
            render_location_filter if locations.any?
            render_category_filter if categories.any?
          end
        end
      end

      def render_location_filter
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'inventory_location_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { t('medications.show.location') }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: current_location_name || t('medications.index.all_locations', default: 'All locations')
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(
                placeholder: t('medications.index.search_locations', default: 'Filter locations')
              )

              render RubyUI::ComboboxList.new do
                render RubyUI::ComboboxEmptyState.new do
                  t('medications.index.no_locations_found', default: 'No locations found')
                end

                render RubyUI::ComboboxItem.new do
                  render RubyUI::ComboboxRadio.new(
                    name: 'location_id',
                    value: '',
                    checked: current_location_id.blank?,
                    data: { action: 'change->form-submit#submitForm' }
                  )
                  span { t('medications.index.all_locations', default: 'All locations') }
                end

                locations.each do |location|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'location_id',
                      value: location.id,
                      checked: current_location_id == location.id,
                      data: { action: 'change->form-submit#submitForm' }
                    )
                    span { location.name }
                  end
                end
              end
            end
          end
        end
      end

      def render_category_filter
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'inventory_category_trigger',
            class: 'text-[10px] font-black uppercase tracking-widest text-muted-foreground ml-1'
          ) { 'Category' }
          render RubyUI::Combobox.new(class: 'w-full') do
            render RubyUI::ComboboxTrigger.new(
              placeholder: current_category.presence || t('medications.index.all')
            )

            render RubyUI::ComboboxPopover.new do
              render RubyUI::ComboboxSearchInput.new(
                placeholder: t('forms.medications.filter_categories')
              )

              render RubyUI::ComboboxList.new do
                render RubyUI::ComboboxEmptyState.new do
                  t('forms.medications.no_categories_found')
                end

                render RubyUI::ComboboxItem.new do
                  render RubyUI::ComboboxRadio.new(
                    name: 'category',
                    value: '',
                    checked: current_category.blank?,
                    data: { action: 'change->form-submit#submitForm' }
                  )
                  span { t('medications.index.all') }
                end

                categories.each do |category|
                  render RubyUI::ComboboxItem.new do
                    render RubyUI::ComboboxRadio.new(
                      name: 'category',
                      value: category,
                      checked: current_category == category,
                      data: { action: 'change->form-submit#submitForm' }
                    )
                    span { category }
                  end
                end
              end
            end
          end
        end
      end

      def current_location_name
        return nil if current_location_id.blank?

        locations.find { |location| location.id == current_location_id }&.name
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
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: 'medications') do
          medications.each do |medication|
            render_medication_card(medication)
          end
        end
      end

      def render_medication_card(medication)
        Card(
          id: "medication_#{medication.id}",
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-surface-container-lowest ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medication_icon
              status_badge(medication)
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
            class: 'flex justify-between items-center text-[10px] font-black uppercase ' \
                   'tracking-widest text-muted-foreground'
          ) do
            span { t('medications.index.inventory_level') }
            span { pluralize(medication.current_supply, 'unit') }
          end
          div(class: 'h-1.5 w-full bg-surface-container-low rounded-full overflow-hidden') do
            div(class: "h-full #{bar_color} rounded-full transition-all duration-1000", style: "width: #{percentage}%")
          end
        end
      end

      def render_medication_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-surface-container-low flex items-center ' \
                 'justify-center text-muted-foreground ' \
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
            class: 'flex-1 rounded-xl py-5 border-border bg-surface-container-lowest ' \
                   'hover:bg-surface-container-low text-muted-foreground'
          ) do
            t('medications.index.view')
          end
          Link(
            href: edit_medication_path(medication, return_to: medications_path(inventory_query_params)),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-border bg-surface-container-lowest ' \
                   'hover:bg-surface-container-low text-muted-foreground',
            aria_label: t('medications.index.edit', default: 'Edit medication')
          ) do
            render Icons::Pencil.new(size: 16)
          end
          if view_context.policy(medication).update?
            refill_classes = if medication.reorder_received?
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0'
                             else
                               'flex items-center justify-center rounded-xl w-10 h-10 p-0 ' \
                                 'border-border bg-surface-container-lowest ' \
                                 'hover:bg-surface-container-low text-muted-foreground'
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
