# frozen_string_literal: true

module MedicationWizardSupport
  extend ActiveSupport::Concern

  private

  def create_success
    return redirect_to(@medication, notice: t('medications.created')) unless params[:wizard] == 'true'

    seed_initial_dosage
    respond_to do |format|
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'wizard-content',
          Components::Medications::Wizard::StepDosages.new(medication: @medication)
        )
      end
      format.html { redirect_to @medication, notice: t('medications.created') }
    end
  end

  def wizard_wrapper_class
    case current_user.wizard_variant
    when 'modal'     then Components::Medications::Wizard::ModalWrapper
    when 'slideover' then Components::Medications::Wizard::SlideOverWrapper
    else                  Components::Medications::Wizard::FullPageWrapper
    end
  end

  def seed_initial_dosage
    return if @medication.dosage_records.exists?
    return unless @medication.dosage_amount.present? && @medication.dosage_unit.present?

    @medication.dosage_records.create!(
      amount: @medication.dosage_amount,
      unit: @medication.dosage_unit,
      frequency: 'As directed',
      default_for_adults: true,
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )
  end
end
