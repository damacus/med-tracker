# frozen_string_literal: true

class PeopleController < ApplicationController
  before_action :set_person, only: %i[show edit update destroy]

  def index
    authorize Person
    people = policy_scope(Person)
             .includes(:user, :schedules, carer_relationships: :carer, location_memberships: :location)
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    render Components::People::ShowView.new(person: @person)
  end

  def new
    @person = Person.new
    authorize @person
    render_modal_or_page(Components::People::FormView.new(person: @person))
  end

  def edit
    authorize @person
    render_modal_or_page(Components::People::FormView.new(person: @person))
  end

  def create
    @person = Person.new(person_params)
    @person.user = current_user
    authorize @person

    if @person.save
      respond_to do |format|
        format.turbo_stream { redirect_to person_path(@person), notice: 'Person was successfully created.' }
        format.html { redirect_to person_path(@person), notice: 'Person was successfully created.' }
      end
    else
      render_modal_or_page(Components::People::FormView.new(person: @person), status: :unprocessable_entity)
    end
  end

  def update
    authorize @person
    if @person.update(person_params)
      respond_to do |format|
        format.turbo_stream { redirect_to person_path(@person), notice: 'Person was successfully updated.' }
        format.html { redirect_to person_path(@person), notice: 'Person was successfully updated.' }
      end
    else
      render_modal_or_page(Components::People::FormView.new(person: @person), status: :unprocessable_entity)
    end
  end

  def destroy
    authorize @person
    @person.destroy!
    redirect_to people_url, notice: 'Person was successfully destroyed.', status: :see_other
  end

  private

  def set_person
    @person = Person.find(params[:id])
  end

  def person_params
    params.require(:person).permit(:name, :person_type, :date_of_birth)
  end

  def render_modal_or_page(component, status: :ok)
    if request.headers['Turbo-Frame'] == 'modal'
      render component, layout: false, status: status
    else
      render component, status: status
    end
  end
end
