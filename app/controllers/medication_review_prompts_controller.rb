# frozen_string_literal: true

class MedicationReviewPromptsController < ApplicationController
  def index
    authorize MedicationReviewPrompt
    render_index
  end

  def update
    prompt = policy_scope(MedicationReviewPrompt).find(params.expect(:id))
    authorize prompt
    prompt.assign_attributes(prompt_params)
    prompt.reviewed_by_membership = current_membership if prompt.practitioner_review_status?

    if prompt.save
      redirect_to medication_review_prompts_path, notice: t('medication_reviews.updated')
    else
      render_index(prompt_with_errors: prompt, status: :unprocessable_content)
    end
  end

  private

  def render_index(prompt_with_errors: nil, status: :ok)
    people = policy_scope(Person)
    MedicationReviewPromptSync.new(people: people).call
    scope = policy_scope(MedicationReviewPrompt).includes(:person, :primary_medication, :interacting_medication)
    prompts = show_hidden? ? scope : scope.visible_by_default
    prompts = prompts.order(:person_id, :created_at, :id).to_a
    replace_prompt_with_errors(prompts, prompt_with_errors)

    render Components::MedicationReviews::IndexView.new(
      prompts: prompts,
      hidden_count: scope.hidden_low_signal.count,
      show_hidden: show_hidden?
    ), status: status
  end

  def replace_prompt_with_errors(prompts, prompt_with_errors)
    return unless prompt_with_errors

    index = prompts.index { |prompt| prompt.id == prompt_with_errors.id }
    prompts[index] = prompt_with_errors if index
  end

  def show_hidden?
    params[:show_hidden] == '1'
  end

  def prompt_params
    params.expect(
      medication_review_prompt: %i[status practitioner_name practitioner_role reviewed_on review_note]
    )
  end
end
