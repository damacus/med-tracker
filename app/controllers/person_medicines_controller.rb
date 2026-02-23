# frozen_string_literal: true

class PersonMedicinesController < ApplicationController
  include TimelineRefreshable

  before_action :set_person
  before_action :set_person_medicine, only: %i[destroy take_medicine reorder]

  def new
    authorize PersonMedicine
    @person_medicine = @person.person_medicines.build
    @medicines = available_medicines

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
    @medicines = available_medicines

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

  def reorder
    authorize @person_medicine, :update?
    @person_medicine.reorder(params[:direction])
    redirect_to person_path(@person)
  end

  def take_medicine
    authorize @person_medicine, :take_medicine?

    # SECURITY: Enforce timing restrictions server-side
    # This prevents bypassing UI-disabled buttons via direct API calls
    unless @person_medicine.can_administer?
      reason = @person_medicine.administration_blocked_reason
      message = reason == :out_of_stock ? 'Cannot take medicine: out of stock' : 'Cannot take medicine: timing restrictions not met'
      respond_to do |format|
        format.html do
          redirect_back_or_to person_path(@person),
                              alert: t('person_medicines.cannot_take_medicine', default: message)
        end
        format.turbo_stream do
          flash.now[:alert] = t('person_medicines.cannot_take_medicine', default: message)
          render turbo_stream: turbo_stream.update('flash',
                                                   Components::Layouts::Flash.new(alert: flash[:alert]))
        end
      end
      return
    end

    @take = @person_medicine.medication_takes.create!(
      taken_at: Time.current,
      amount_ml: params[:amount_ml] || @person_medicine.medicine.dosage_amount
    )
    flash.now[:notice] = t('person_medicines.medicine_taken')

    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medicines.medicine_taken') }

      format.turbo_stream do
        flash.now[:notice] = t('person_medicines.medicine_taken')
        streams = build_timeline_streams_for(@person_medicine.reload, @take)
        streams << turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
        render turbo_stream: streams
      end
    end
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
    authorize @person, :show?
  end

  def set_person_medicine
    @person_medicine = @person.person_medicines.find(params[:id])
  end

  def person_medicine_params
    params.expect(person_medicine: %i[medicine_id notes max_daily_doses min_hours_between_doses])
  end

  def available_medicines
    Medicine.order(:name)
  end
end
