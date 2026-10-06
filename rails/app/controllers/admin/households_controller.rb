# frozen_string_literal: true

module Admin
  class HouseholdsController < BaseController
    def edit
      @household = current_household
      authorize @household
      render Components::Admin::Households::EditView.new(household: @household)
    end

    def update
      @household = current_household
      authorize @household

      if @household.update(household_params)
        redirect_to edit_admin_household_path, notice: t('admin.households.updated')
      else
        render Components::Admin::Households::EditView.new(household: @household), status: :unprocessable_content
      end
    end

    private

    def household_params
      params.expect(household: [:name])
    end
  end
end
