# frozen_string_literal: true

class Components::Dashboard::PersonSchedule < Components::Base
  include Phlex::Rails::Helpers::ButtonTo
  attr_reader :user, :prescriptions, :take_medicine_url_generator

  def initialize(user:, prescriptions:, take_medicine_url_generator: nil)
    @user = user
    @prescriptions = prescriptions
    @take_medicine_url_generator = take_medicine_url_generator
    super()
  end

  def view_template
    div(class: "schedule-person") do
      render_person_header
      render_prescriptions_list
    end
  end

  private

  def render_person_header
    div(class: "schedule-person__header") do
      h3(class: "schedule-person__name") { user.name }
      p(class: "schedule-person__age") { "Age: #{user.age}" }
    end
  end

  def render_prescriptions_list
    div(class: "schedule-prescriptions") do
      prescriptions.each do |prescription|
        render_prescription_card(prescription)
      end
    end
  end

  def render_prescription_card(prescription)
    div(id: "prescription_#{prescription.id}", class: "prescription-card") do
      div(class: "prescription-card__content") do
        render_prescription_info(prescription)
        render_prescription_actions(prescription)
      end
    end
  end

  def render_prescription_info(prescription)
    div(class: "prescription-card__info") do
      h4(class: "prescription-card__medicine") { prescription.medicine.name }
      p(class: "prescription-card__detail") { "Dosage: #{prescription.dosage&.amount} #{prescription.dosage&.unit}" }
      p(class: "prescription-card__detail") { "Frequency: #{prescription.frequency}" } if prescription.frequency.present?
      
      if prescription.end_date
        p(class: "prescription-card__detail") { "Ends: #{prescription.end_date.strftime('%B %d, %Y')}" }
      end
    end
  end

  def render_prescription_actions(prescription)
    div(class: "prescription-card__actions") do
      if take_medicine_url_generator
        url = take_medicine_url_generator.call(prescription)
        button_to(
          url,
          method: :post,
          class: "quick-action__button",
          data: { test_id: "take-medicine-#{prescription.id}" }
        ) do
          "Take Now"
        end
      else
        # For testing or when URL generator not provided
        button(class: "quick-action__button", data: { test_id: "take-medicine-#{prescription.id}" }) do
          "Take Now"
        end
      end
    end
  end
end
