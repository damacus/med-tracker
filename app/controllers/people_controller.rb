# frozen_string_literal: true

class PeopleController < ApplicationController
  before_action :set_person, only: %i[show update destroy]

  def index
    authorize Person
    people = policy_scope(Person).includes(:user)
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    prescriptions = @person.prescriptions.includes(:medicine, :dosage)
    person_medicines = @person.person_medicines.includes(:medicine).ordered
    editing = params[:editing] == 'true'

    today_start = Time.current.beginning_of_day
    prescription_ids = prescriptions.map(&:id)
    person_medicine_ids = person_medicines.map(&:id)

    takes_by_prescription = MedicationTake
                            .where(prescription_id: prescription_ids, taken_at: today_start..)
                            .order(taken_at: :desc)
                            .group_by(&:prescription_id)

    takes_by_person_medicine = MedicationTake
                               .where(person_medicine_id: person_medicine_ids, taken_at: today_start..)
                               .order(taken_at: :desc)
                               .group_by(&:person_medicine_id)

    render Components::People::ShowView.new(
      person: @person,
      prescriptions: prescriptions,
      person_medicines: person_medicines,
      editing: editing,
      preloaded_takes: {
        prescriptions: takes_by_prescription,
        person_medicines: takes_by_person_medicine
      },
      current_user: current_user
    )
  end

  def new
    @person = Person.new
    authorize @person
    render Components::People::FormView.new(person: @person)
  end

  def edit
    @person = Person.find(params[:id])
    authorize @person
    render Components::People::FormView.new(person: @person)
  end

  def create
    @person = Person.new(person_params)
    authorize @person

    if current_user.parent? || current_user.carer?
      @person.carer_relationships.build(
        carer: current_user.person,
        relationship_type: current_user.role,
        active: true
      )
    end

    respond_to do |format|
      if @person.save
        format.html { redirect_to @person, notice: t('people.created') }
        format.turbo_stream { redirect_to people_path, notice: t('people.created') }
      else
        format.html do
          render Components::People::FormView.new(person: @person), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'person_form',
            Components::People::FormView.new(person: @person)
          )
        end
      end
    end
  end

  def update
    authorize @person
    respond_to do |format|
      if @person.update(person_params)
        format.html { redirect_back_or_to @person, notice: t('people.updated') }
        format.turbo_stream { redirect_back_or_to people_path, notice: t('people.updated') }
        format.json { render :show, status: :ok, location: @person }
      else
        format.html do
          render Components::People::FormView.new(person: @person), status: :unprocessable_content
        end
        format.json { render json: @person.errors, status: :unprocessable_content }
      end
    end
  end

  # DELETE /people/1 or /people/1.json
  def destroy
    authorize @person
    @person.destroy!

    respond_to do |format|
      format.html { redirect_to people_path, status: :see_other, notice: t('people.deleted') }
      format.json { head :no_content }
    end
  end

  private

  def set_person
    @person = Person.find(params[:id])
  end

  def person_params
    params.expect(person: %i[name date_of_birth email person_type has_capacity])
  end
end
