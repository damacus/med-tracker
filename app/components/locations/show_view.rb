# frozen_string_literal: true

module Components
  module Locations
    class ShowView < Components::Base
      attr_reader :location, :notice

      def initialize(location:, notice: nil)
        @location = location
        @notice = notice
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-6xl space-y-12') do
          render_notice if notice.present?
          render_header

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-12') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_medicines_section
            end

            div(class: 'space-y-8') do
              render_members_card
              render_details_card
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
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-slate-100') do
          div(class: 'flex items-center gap-6') do
            div(
              class: 'w-20 h-20 rounded-[2rem] bg-primary/10 flex items-center justify-center text-primary shadow-inner'
            ) do
              render Icons::Home.new(size: 32)
            end
            div(class: 'space-y-1') do
              Text(size: '2', weight: 'black', class: 'uppercase tracking-[0.2em] font-bold opacity-40 block mb-1') do
                'Location'
              end
              Heading(level: 1, size: '8', class: 'font-black tracking-tight') { location.name }
            end
          end

          div(class: 'flex gap-3') do
            Link(href: edit_location_path(location), variant: :outline, size: :lg,
                 class: 'rounded-2xl font-bold text-sm bg-white') do
              'Edit Location'
            end
            Link(href: locations_path, variant: :ghost, size: :lg,
                 class: 'rounded-2xl font-bold text-sm text-slate-400 hover:text-slate-600') do
              'All Locations'
            end
          end
        end
      end

      def render_medicines_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center justify-between') do
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { 'Medicines at this Location' }
          end

          if location.medicines.any?
            div(class: 'grid grid-cols-1 md:grid-cols-2 gap-4') do
              location.medicines.each do |medicine|
                render_medicine_card(medicine)
              end
            end
          else
            Card(class: 'p-8 text-center') do
              Text(size: '3', class: 'text-slate-400') { 'No medicines at this location yet.' }
            end
          end
        end
      end

      def render_medicine_card(medicine)
        Card(class: 'p-6 hover:shadow-md transition-shadow') do
          div(class: 'flex items-center justify-between') do
            div(class: 'flex items-center gap-4') do
              div(class: 'w-10 h-10 rounded-xl bg-slate-50 flex items-center justify-center text-slate-400') do
                render Icons::Pill.new(size: 20)
              end
              div do
                Text(size: '3', weight: 'semibold') { medicine.name }
                if medicine.dosage_amount.present? && medicine.dosage_unit.present?
                  Text(size: '1', class: 'text-slate-400') { "#{medicine.dosage_amount} #{medicine.dosage_unit}" }
                end
              end
            end
            if medicine.low_stock?
              Badge(variant: :destructive) { 'Low Stock' }
            else
              Badge(variant: :success) { "#{medicine.current_supply} units" }
            end
          end
        end
      end

      def render_members_card
        Card(class: 'p-8 space-y-6') do
          Heading(level: 3, size: '4', class: 'font-bold') { 'Members' }

          if location.members.any?
            div(class: 'space-y-3') do
              location.members.each do |member|
                div(class: 'flex items-center gap-3') do
                  div(class: 'w-8 h-8 rounded-full bg-slate-100 flex items-center justify-center text-slate-500') do
                    render Icons::User.new(size: 16)
                  end
                  Text(size: '2', weight: 'medium') { member.name }
                end
              end
            end
          else
            Text(size: '2', class: 'text-slate-400 italic') { 'No members assigned.' }
          end
        end
      end

      def render_details_card
        Card(class: 'p-8 space-y-4') do
          Heading(level: 3, size: '4', class: 'font-bold') { 'Details' }

          if location.description.present?
            Text(size: '2', class: 'text-slate-600 leading-relaxed') { location.description }
          else
            Text(size: '2', class: 'text-slate-400 italic') { 'No description provided.' }
          end
        end
      end
    end
  end
end
