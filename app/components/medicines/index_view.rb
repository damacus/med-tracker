# frozen_string_literal: true

module Components
  module Medicines
    class IndexView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :medicines

      def initialize(medicines:)
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'medicines-list' }) do
          render_header
          render_categories_section
          render_medicines_grid
        end
      end

      private

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
          div do
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              'Your Inventory'
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { 'Medicines' }
          end
          if view_context.policy(Medicine).create?
            Link(
              href: new_medicine_path,
              variant: :primary,
              size: :lg,
              class: 'rounded-2xl font-bold text-sm shadow-lg shadow-primary/20',
              data: { turbo_stream: true }
            ) do
              render Icons::Pill.new(size: 20, class: 'mr-2')
              span { 'Add Medicine' }
            end
          end
        end
      end

      def render_categories_section
        categories = medicines.filter_map(&:category).uniq.sort
        return if categories.empty?

        div(class: 'mb-12 overflow-x-auto no-scrollbar') do
          div(class: 'flex gap-3 min-w-max pb-2') do
            Button(
              variant: :primary,
              class: 'rounded-full px-6 py-2 h-auto text-xs font-bold uppercase tracking-wider ' \
                     'shadow-md shadow-primary/10'
            ) { 'All' }

            categories.each do |cat|
              Button(
                variant: :outline,
                class: 'rounded-full px-6 py-2 h-auto text-xs font-bold uppercase tracking-wider'
              ) { cat.pluralize.titleize }
            end
          end
        end
      end

      def render_medicines_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8', id: 'medicines') do
          medicines.each do |medicine|
            render_medicine_card(medicine)
          end
        end
      end

      def render_medicine_card(medicine)
        Card(
          id: "medicine_#{medicine.id}",
          class: 'h-full flex flex-col border-none shadow-[0_8px_30px_rgb(0,0,0,0.04)] bg-white ' \
                 'rounded-[2.5rem] transition-all duration-300 hover:scale-[1.02] hover:shadow-xl ' \
                 'group overflow-hidden'
        ) do
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medicine_icon
              status_badge(medicine)
            end
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { medicine.name }
          end

          CardContent(class: 'flex-grow space-y-6 px-8 pb-4') do
            if medicine.description.present?
              Text(size: '2', class: 'text-slate-400 line-clamp-2 leading-relaxed') { medicine.description }
            end

            div(class: 'pt-4 border-t border-slate-50 space-y-4') do
              render_supply_bar(medicine)
            end
          end

          CardFooter(class: 'px-8 pb-8 pt-2 mt-auto') do
            render_medicine_actions(medicine)
          end
        end
      end

      def status_badge(medicine)
        if medicine.low_stock?
          Badge(variant: :destructive) { 'Low Stock' }
        else
          Badge(variant: :success) { 'In Stock' }
        end
      end

      def render_supply_bar(medicine)
        percentage = medicine.supply_percentage
        bar_color = medicine.low_stock? ? 'bg-destructive' : 'bg-primary'

        div(class: 'space-y-2') do
          div(
            class: 'flex justify-between items-center text-[10px] font-black uppercase tracking-widest text-slate-400'
          ) do
            span { 'Inventory Level' }
            span { "#{medicine.current_supply} units" }
          end
          div(class: 'h-1.5 w-full bg-slate-50 rounded-full overflow-hidden') do
            div(class: "h-full #{bar_color} rounded-full transition-all duration-1000", style: "width: #{percentage}%")
          end
        end
      end

      def render_medicine_icon
        div(
          class: 'w-12 h-12 rounded-2xl bg-slate-50 flex items-center justify-center text-slate-400 ' \
                 'group-hover:text-primary group-hover:bg-primary/5 transition-all'
        ) do
          render Icons::Pill.new(size: 24)
        end
      end

      def render_medicine_actions(medicine)
        div(class: 'flex items-center gap-2 w-full') do
          Link(
            href: medicine_path(medicine),
            variant: :outline,
            size: :sm,
            class: 'flex-1 rounded-xl py-5 border-slate-100 bg-white hover:bg-slate-50 text-slate-600'
          ) do
            'View'
          end
          Link(
            href: edit_medicine_path(medicine, return_to: medicines_path),
            variant: :outline,
            size: :sm,
            class: 'rounded-xl w-10 h-10 p-0 border-slate-100 bg-white hover:bg-slate-50 text-slate-400'
          ) do
            render Icons::Pencil.new(size: 16)
          end
          if view_context.policy(medicine).update?
            render Components::Medicines::RefillModal.new(
              medicine: medicine,
              button_variant: :outline,
              button_class: 'flex items-center justify-center rounded-xl w-10 h-10 p-0 ' \
                            'border-slate-100 bg-white hover:bg-slate-50 text-slate-400',
              icon_only: true
            )
          end
          render_delete_dialog(medicine)
        end
      end

      def render_delete_dialog(medicine)
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :ghost, size: :sm,
                   class: 'rounded-xl w-10 h-10 p-0 text-slate-300 hover:text-destructive hover:bg-destructive/5') do
              render Icons::Trash.new(size: 18)
            end
          end
          AlertDialogContent(class: 'rounded-[2rem] border-none shadow-2xl') do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Medicine' }
              AlertDialogDescription do
                "Are you sure you want to delete #{medicine.name}? This action cannot be undone."
              end
            end
            AlertDialogFooter do
              AlertDialogCancel(class: 'rounded-xl') { 'Cancel' }
              form_with(url: medicine_path(medicine), method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit, class: 'rounded-xl shadow-lg shadow-destructive/20') do
                  'Delete'
                end
              end
            end
          end
        end
      end
    end
  end
end
