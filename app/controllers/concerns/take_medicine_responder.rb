# frozen_string_literal: true

module TakeMedicineResponder
  extend ActiveSupport::Concern

  private

  def handle_take_medicine(takeable:, i18n_scope:)
    result = MedicineAdministrationService.call(takeable: takeable, amount_ml: params[:amount_ml])

    if result.failure?
      respond_to_take_failure(i18n_scope: i18n_scope, message: result.message)
    else
      respond_to_take_success(takeable: takeable, i18n_scope: i18n_scope)
    end
  end

  def respond_to_take_failure(i18n_scope:, message:)
    respond_to do |format|
      format.html do
        redirect_back_or_to person_path(@person),
                            alert: t("#{i18n_scope}.cannot_take_medicine", default: message)
      end
      format.turbo_stream do
        flash.now[:alert] = t("#{i18n_scope}.cannot_take_medicine", default: message)
        render turbo_stream: turbo_stream.update('flash',
                                                 Components::Layouts::Flash.new(alert: flash[:alert]))
      end
    end
  end

  def respond_to_take_success(takeable:, i18n_scope:)
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t("#{i18n_scope}.medicine_taken") }
      format.turbo_stream do
        flash.now[:notice] = t("#{i18n_scope}.medicine_taken")
        render turbo_stream: take_success_turbo_streams(takeable)
      end
    end
  end

  def take_success_turbo_streams(takeable)
    raise NotImplementedError
  end
end
