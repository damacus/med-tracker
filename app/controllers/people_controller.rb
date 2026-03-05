# frozen_string_literal: true

class PeopleController < ApplicationController
  include PersonViewable

  INDEX_PRELOADS = [:user, :schedules, { carer_relationships: :carer }, { location_memberships: :location }].freeze
  SHOW_PRELOADS = [:user, :schedules, { person_medications: :medication }, { location_memberships: :location }].freeze

  before_action :set_person, only: %i[show edit update destroy]

  def index
    authorize Person
    people = policy_scope(Person).includes(INDEX_PRELOADS).order(:name)
    render Components::People::IndexView.new(people: people, current_user: current_user)
  end

  def show
    authorize @person
    render Components::People::ShowView.new(
      person: @person, 
      schedules: @person.schedules.includes(:medication, :dosage),
      person_medications: @person.person_medications.includes(:medication),
      current_user: current_user
    )
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
    @person.user = current_user if person_params[:person_type] == 'adult'
    @person.primary_location = current_user.person&.locations&.first
    
    # Auto-link current user as carer for dependents
    if %w[minor dependent_adult].include?(person_params[:person_type])
      @person.carer_relationships.build(carer: current_user.person, relationship_type: 'parent', active: true)
    end

    authorize @person

    if @person.save
      respond_to do |format|
        format.turbo_stream do
          flash.now[:notice] = t('people.create_success')
          render turbo_stream: [
            turbo_stream.append('people', Components::People::PersonCard.new(person: @person, current_user: current_user)),
            turbo_stream.replace('modal', ''),
            turbo_stream.prepend('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
          ]
        end
        format.html { redirect_to person_path(@person), notice: t('people.create_success') }
      end
    else
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace('modal', Components::People::FormView.new(person: @person)), 
                 status: :unprocessable_entity
        end
        format.html { render_modal_or_page(Components::People::FormView.new(person: @person), status: :unprocessable_entity) }
      end
    end
  end

  def update
    authorize @person
    if @person.update(person_params)
      respond_to do |format|
        format.turbo_stream do
          flash.now[:notice] = t('people.update_success')
          render turbo_stream: [
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person, current_user: current_user)),
            turbo_stream.replace("person_show_#{@person.id}", Components::People::ShowView.new(
              person: @person,
              schedules: @person.schedules.includes(:medication, :dosage),
              person_medications: @person.person_medications.includes(:medication),
              current_user: current_user
            )),
            turbo_stream.replace('modal', ''),
            turbo_stream.prepend('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
          ]
        end
        format.html { redirect_to person_path(@person), notice: t('people.update_success') }
      end
    else
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace('modal', Components::People::FormView.new(person: @person)), 
                 status: :unprocessable_entity
        end
        format.html { render_modal_or_page(Components::People::FormView.new(person: @person), status: :unprocessable_entity) }
      end
    end
  end

  def destroy
    authorize @person
    @person.destroy!
    redirect_to people_url, notice: t('people.destroy_success'), status: :see_other
  end

  private

  def set_person
    @person = Person.find(params[:id])
  end

  def person_params
    params.require(:person).permit(:name, :person_type, :date_of_birth, :email)
  end

  def render_modal_or_page(component, status: :ok)
    if request.headers['Turbo-Frame'] == 'modal'
      render component, layout: false, status: status
    else
      render component, status: status
    end
  end
end
