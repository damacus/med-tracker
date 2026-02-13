# frozen_string_literal: true

class PersonMedicinesController < ApplicationController
  include PersonScoped
  include TakeMedicineResponder

  before_action :set_person_medicine, only: %i[destroy take_medicine]

  def new
    authorize PersonMedicine
    @person_medicine = @person.person_medicines.build
    @medicines = Medicine.all

    respond_to do |format|
      format.html do
        render Components::PersonMedicines::FormView.new(
          person_medicine: @person_medicine,
          person: @person,
          medicines: @medicines
        )
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'person_medicine_modal',
          Components::PersonMedicines::Modal.new(
            person_medicine: @person_medicine,
            person: @person,
            medicines: @medicines,
            title: t('person_medicines.modal.new_title', person: @person.name)
          )
        )
      end
    end
  end

  def create
    @person_medicine = @person.person_medicines.build(person_medicine_params)
    authorize @person_medicine
    @medicines = Medicine.all

    if @person_medicine.save
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('person_medicines.created') }
        format.turbo_stream do
          flash.now[:notice] = t('person_medicines.created')
          render turbo_stream: [
            turbo_stream.remove('person_medicine_modal'),
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      respond_to do |format|
        format.html do
          render Components::PersonMedicines::FormView.new(
            person_medicine: @person_medicine,
            person: @person,
            medicines: @medicines
          ), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.update(
            'person_medicine_modal',
            Components::PersonMedicines::Modal.new(
              person_medicine: @person_medicine,
              person: @person,
              medicines: @medicines,
              title: t('person_medicines.modal.new_title', person: @person.name)
            )
          ), status: :unprocessable_content
        end
      end
    end
  end

  def destroy
    authorize @person_medicine
    @person_medicine.destroy
    redirect_to person_path(@person), notice: t('person_medicines.deleted')
  end

  def take_medicine
    authorize @person_medicine, :take_medicine?
    handle_take_medicine(takeable: @person_medicine, i18n_scope: 'person_medicines')
  end

  def take_success_turbo_streams(_takeable)
    [
      turbo_stream.replace("person_#{@person.id}",
                           Components::People::PersonCard.new(person: @person.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
    ]
  end

  private

  def set_person
    super
    authorize @person, :show?
  end

  def set_person_medicine
    @person_medicine = @person.person_medicines.find(params[:id])
  end

  def person_medicine_params
    params.expect(person_medicine: %i[medicine_id notes max_daily_doses min_hours_between_doses])
  end
end
