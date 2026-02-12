# frozen_string_literal: true

module Components
  module Shared
    class DeleteConfirmDialog < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :title, :description, :delete_url, :trigger_label, :confirm_label

      def initialize(title:, description:, delete_url:, trigger_label: 'Delete', confirm_label: 'Delete')
        @title = title
        @description = description
        @delete_url = delete_url
        @trigger_label = trigger_label
        @confirm_label = confirm_label
        super()
      end

      def view_template
        AlertDialog do
          AlertDialogTrigger do
            Button(variant: :destructive_outline, size: :md) { trigger_label }
          end
          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { title }
              AlertDialogDescription { description }
            end
            AlertDialogFooter do
              AlertDialogCancel { 'Cancel' }
              form_with(url: delete_url, method: :delete, class: 'inline') do
                Button(variant: :destructive, type: :submit) { confirm_label }
              end
            end
          end
        end
      end
    end
  end
end
