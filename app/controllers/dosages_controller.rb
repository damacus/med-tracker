# frozen_string_literal: true

class DosagesController < ApplicationController
  before_action :set_medication
  before_action :set_dosage, only: %i[edit update destroy]

  def new
    @dosage = @medication.dosages.build
    authorize @dosage
    render Components::Dosages::Modal.new(dosage: @dosage, medication: @medication)
  end

  def edit
    authorize @dosage
    render Components::Dosages::Modal.new(dosage: @dosage, medication: @medication)
  end

  def create
    @dosage = @medication.dosages.build(dosage_params)
    authorize @dosage

    if @dosage.save
      if params[:wizard] == 'true'
        respond_to do |format|
          format.turbo_stream do
            dosage_row = Components::Medications::Wizard::DosageRow.new(dosage: @dosage)
            form_frame = Components::Medications::Wizard::DosageFormFrame.new(medication: @medication)
            render turbo_stream: [
              turbo_stream.append('dosage-list', dosage_row),
              turbo_stream.replace('dosage-form', form_frame)
            ]
          end
          format.html { redirect_to @medication, notice: t('dosages.created') }
        end
      else
        redirect_to @medication, notice: t('dosages.created')
      end
    elsif params[:wizard] == 'true'
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'dosage-form',
            Components::Medications::Wizard::DosageFormFrame.new(
              medication: @medication,
              dosage: @dosage
            )
          ), status: :unprocessable_content
        end
        format.html { redirect_to @medication }
      end
    else
      render Components::Dosages::Modal.new(dosage: @dosage, medication: @medication),
             status: :unprocessable_content
    end
  end

  def update
    authorize @dosage

    if @dosage.update(dosage_params)
      redirect_to @medication, notice: t('dosages.updated')
    else
      render Components::Dosages::Modal.new(dosage: @dosage, medication: @medication),
             status: :unprocessable_content
    end
  end

  def destroy
    authorize @dosage
    @dosage.destroy
    redirect_to @medication, notice: t('dosages.deleted')
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find(params[:medication_id])
  end

  def set_dosage
    @dosage = @medication.dosages.find(params[:id])
  end

  def dosage_params
    params.expect(
      dosage: %i[amount unit frequency description
                 default_for_adults default_for_children
                 default_max_daily_doses default_min_hours_between_doses
                 default_dose_cycle]
    )
  end
end
