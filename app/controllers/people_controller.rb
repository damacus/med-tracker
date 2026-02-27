# frozen_string_literal: true

class PeopleController < ApplicationController
  include PersonViewable

  before_action :set_person, only: %i[show update destroy]

  def index
    authorize Person
    people = policy_scope(Person).includes(:user)
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    schedules = @person.schedules.includes(:medication, :dosage)
    person_medications = @person.person_medications.includes(:medication).ordered
    editing = params[:editing] == 'true'

    today_start = Time.current.beginning_of_day
    schedule_ids = schedules.map(&:id)
    person_medication_ids = person_medications.map(&:id)

    takes_by_schedule = MedicationTake
                        .where(schedule_id: schedule_ids, taken_at: today_start..)
                        .order(taken_at: :desc)
                        .group_by(&:schedule_id)

    takes_by_person_medication = MedicationTake
                                 .where(person_medication_id: person_medication_ids, taken_at: today_start..)
                                 .order(taken_at: :desc)
                                 .group_by(&:person_medication_id)

    render Components::People::ShowView.new(
      person: @person,
      schedules: schedules,
      person_medications: person_medications,
      editing: editing,
      preloaded_takes: {
        schedules: takes_by_schedule,
        person_medications: takes_by_person_medication
      },
      current_user: current_user
    )
  end

  def new
    @person = Person.new
    authorize @person
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    if is_modal
      render Components::People::FormView.new(person: @person), layout: false
    else
      render Components::People::FormView.new(person: @person) # FormView already handles its own layout
    end
  end

  def edit
    @person = Person.find(params[:id])
    authorize @person
    @return_to = params[:return_to]
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    if is_modal
      render Components::People::FormView.new(person: @person, return_to: @return_to), layout: false
    else
      render Components::People::FormView.new(person: @person, return_to: @return_to)
    end
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
        format.turbo_stream do
          flash.now[:notice] = t('people.created')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.prepend('people', Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      else
        format.html do
          render Components::People::FormView.new(person: @person), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::FormView.new(person: @person)
          ), status: :unprocessable_content
        end
      end
    end
  end

  def update
    authorize @person
    respond_to do |format|
      if @person.update(person_params)
        format.html { redirect_to params[:return_to].presence || @person, notice: t('people.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('people.updated')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
        format.json { render :show, status: :ok, location: @person }
      else
        format.html do
          render Components::People::FormView.new(person: @person, return_to: params[:return_to]), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::FormView.new(person: @person, return_to: params[:return_to])
          ), status: :unprocessable_content
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
