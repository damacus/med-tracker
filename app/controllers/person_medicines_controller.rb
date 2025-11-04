# frozen_string_literal: true

class PersonMedicinesController < ApplicationController
  before_action :set_person
  before_action :set_person_medicine, only: %i[destroy take_medicine]

  def new
    authorize PersonMedicine
    @person_medicine = @person.person_medicines.build
    @medicines = Medicine.all

    respond_to do |format|
      format.html { render :new, locals: { inline: false, medicines: @medicines, person: @person } }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', partial: 'shared/modal', locals: {
                                                    title: "Add Medicine for #{@person.name}",
                                                    content: render_to_string(
                                                      partial: 'form',
                                                      locals: { person_medicine: @person_medicine, inline: true, medicines: @medicines, person: @person }
                                                    )
                                                  })
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
            turbo_stream.remove('modal'),
            turbo_stream.replace("person_#{@person.id}", partial: 'people/person', locals: { person: @person.reload }),
            turbo_stream.update('flash', partial: 'shared/flash')
          ]
        end
      end
    else
      respond_to do |format|
        format.html { render :new, status: :unprocessable_entity, locals: { medicines: @medicines, person: @person } }
        format.turbo_stream do
          render turbo_stream: turbo_stream.update('modal', partial: 'shared/modal', locals: {
                                                     title: "Add Medicine for #{@person.name}",
                                                     content: render_to_string(
                                                       partial: 'form',
                                                       locals: { person_medicine: @person_medicine, inline: true, medicines: @medicines, person: @person }
                                                     )
                                                   }), status: :unprocessable_entity
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
    @take = @person_medicine.medication_takes.create!(
      taken_at: Time.current,
      amount_ml: params[:amount_ml] || @person_medicine.medicine.dosage_amount
    )
    redirect_to person_path(@person), notice: t('person_medicines.medicine_taken')
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
end
