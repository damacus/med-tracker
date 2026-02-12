# frozen_string_literal: true

module Components
  module Admin
    module Users
      # Renders pagination controls for the users list
      # Delegates to the shared Pagination component
      class Pagination < Components::Base
        attr_reader :pagy_obj, :search_params

        def initialize(pagy:, search_params: {})
          @pagy_obj = pagy
          @search_params = search_params
          super()
        end

        def view_template
          render Components::Shared::Pagination.new(
            pagy: pagy_obj,
            base_url: '/admin/users',
            extra_params: search_params
          )
        end
      end
    end
  end
end
