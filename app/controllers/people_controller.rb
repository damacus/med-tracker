# frozen_string_literal: true

class PeopleController < ApplicationController
  include PersonViewable

  before_action :set_person, only: %i[show update destroy add_medication]

  def index
    authorize Person
    people = PeopleIndexQuery.new(scope: policy_scope(Person)).call
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    show_data = PersonShowQuery.new(person: @person).call
    editing = params[:editing] == 'true'

    render Components::People::ShowView.new(
      person: @person,
      schedules: show_data.schedules,
      person_medications: show_data.person_medications,
      editing: editing,
      preloaded_takes: show_data.preloaded_takes,
      current_user: current_user
    )
  end

  def new
    @person = Person.new
    authorize @person
    is_modal = request.headers['Turbo-Frame'] == 'modal'
    assigned_location = primary_location

    if is_modal
      render Components::People::Modal.new(person: @person, assigned_location: assigned_location), layout: false
    else
      render Components::People::FormView.new(person: @person, assigned_location: assigned_location)
    end
  end

  def edit
    @person = policy_scope(Person).find(params[:id])
    authorize @person
    @return_to = params[:return_to]
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    if is_modal
      render Components::People::Modal.new(person: @person, return_to: @return_to), layout: false
    else
      render Components::People::FormView.new(person: @person, return_to: @return_to)
    end
  end

  def create
    @person = Person.new(person_params)
    authorize @person
    @person.primary_location = primary_location

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
          render Components::People::FormView.new(person: @person, assigned_location: primary_location),
                 status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::Modal.new(person: @person, assigned_location: primary_location)
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
            Components::People::Modal.new(person: @person, return_to: params[:return_to])
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

  def add_medication
    authorize @person, :show?
    back = params[:source] == 'workflow' ? add_medication_path : nil
    render Components::People::AddMedicationLanding.new(
      person: @person,
      can_schedule: policy(Schedule.new(person: @person)).create?,
      can_person_medication: policy(PersonMedication.new(person: @person)).create?,
      back_path: back,
      medication_id: params[:medication_id]
    )
  end

  private

  def set_person
    @person = policy_scope(Person).find(params[:id])
  end

  def person_params
    params.expect(person: %i[name date_of_birth email person_type has_capacity])
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end
end
