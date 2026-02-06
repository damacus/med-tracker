# frozen_string_literal: true

module Components
  module Admin
    module CarerRelationships
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::ButtonTo
        include Phlex::Rails::Helpers::FormWith

        attr_reader :relationships, :current_user, :pagy_obj

        def initialize(relationships:, current_user: nil, pagy: nil)
          @relationships = relationships
          @current_user = current_user
          @pagy_obj = pagy
          super()
        end

        def view_template
          div(data: { testid: 'admin-carer-relationships' }, class: 'space-y-8 px-4 sm:px-6 lg:px-8') do
            render_header
            render_relationships_table
            render_pagination if pagy_obj && pagy_obj.pages > 1
          end
        end

        private

        def render_header
          header(class: 'flex items-center justify-between') do
            div(class: 'space-y-2') do
              Heading(level: 1) { 'Carer Relationships' }
              Text(weight: 'muted') { 'Manage carer-patient assignments.' }
            end
            render RubyUI::Link.new(href: '/admin/carer_relationships/new', variant: :primary) { 'New Relationship' }
          end
        end

        def render_relationships_table
          div(class: 'rounded-xl border border-border bg-card shadow-sm') do
            render RubyUI::Table.new do
              render_table_header
              render_table_body
            end
          end
        end

        def render_table_header
          render RubyUI::TableHeader.new do
            render RubyUI::TableRow.new do
              render(RubyUI::TableHead.new { 'Carer' })
              render(RubyUI::TableHead.new { 'Patient' })
              render(RubyUI::TableHead.new { 'Type' })
              render(RubyUI::TableHead.new { 'Status' })
              render RubyUI::TableHead.new(class: 'text-right') { 'Actions' }
            end
          end
        end

        def render_table_body
          render RubyUI::TableBody.new do
            if relationships.empty?
              render RubyUI::TableRow.new do
                render RubyUI::TableCell.new(colspan: 5, class: 'py-8 text-center text-muted-foreground') do
                  'No carer relationships found.'
                end
              end
            else
              relationships.each do |relationship|
                render_relationship_row(relationship)
              end
            end
          end
        end

        def render_relationship_row(relationship)
          row_class = relationship.active? ? '' : 'opacity-60'
          render RubyUI::TableRow.new(class: row_class, data: { relationship_id: relationship.id }) do
            render(RubyUI::TableCell.new(class: 'font-medium') { relationship.carer.name })
            render(RubyUI::TableCell.new { relationship.patient.name })
            render(RubyUI::TableCell.new(class: 'capitalize') { relationship.relationship_type.to_s.humanize })
            render(RubyUI::TableCell.new { render_status_badge(relationship) })
            render RubyUI::TableCell.new(class: 'text-right space-x-2') do
              render_activation_button(relationship)
            end
          end
        end

        def render_status_badge(relationship)
          if relationship.active?
            render RubyUI::Badge.new(variant: :green) { 'Active' }
          else
            render RubyUI::Badge.new(variant: :red) { 'Inactive' }
          end
        end

        def render_activation_button(relationship)
          if relationship.active?
            render_deactivate_dialog(relationship)
          else
            form_with(
              url: "/admin/carer_relationships/#{relationship.id}/activate",
              method: :post,
              class: 'inline-block'
            ) do
              Button(
                type: :submit,
                variant: :outline,
                size: :sm,
                class: 'text-green-600 hover:text-green-500 border-green-600'
              ) { 'Activate' }
            end
          end
        end

        def render_deactivate_dialog(relationship)
          render RubyUI::AlertDialog.new do
            render RubyUI::AlertDialogTrigger.new do
              Button(variant: :outline, size: :sm, class: 'text-red-600 hover:bg-red-50 hover:text-red-700') do
                'Deactivate'
              end
            end
            render RubyUI::AlertDialogContent.new do
              render RubyUI::AlertDialogHeader.new do
                render(RubyUI::AlertDialogTitle.new { 'Deactivate Relationship' })
                render RubyUI::AlertDialogDescription.new do
                  "Are you sure you want to deactivate the relationship between #{relationship.carer.name} " \
                    "and #{relationship.patient.name}?"
                end
              end
              render RubyUI::AlertDialogFooter.new do
                render(RubyUI::AlertDialogCancel.new { 'Cancel' })
                form_with(url: "/admin/carer_relationships/#{relationship.id}", method: :delete, class: 'inline') do
                  Button(variant: :destructive, type: :submit) { 'Deactivate' }
                end
              end
            end
          end
        end

        def render_pagination
          div(class: 'flex items-center justify-between border-t border-slate-200 bg-white px-4 py-3 sm:px-6') do
            div(data: { testid: 'pagination-info' }) do
              Text(size: '2', class: 'text-slate-700') do
                plain 'Showing '
                span(class: 'font-medium') { pagy_obj.from.to_s }
                plain ' to '
                span(class: 'font-medium') { pagy_obj.to.to_s }
                plain ' of '
                span(class: 'font-medium') { pagy_obj.count.to_s }
                plain ' results'
              end
            end
          end
        end
      end
    end
  end
end
