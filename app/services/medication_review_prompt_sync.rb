# frozen_string_literal: true

class MedicationReviewPromptSync
  def initialize(people:, evidence_scope: MedicationReviewEvidenceRecord.detectable)
    @people = people
    @evidence_scope = evidence_scope
  end

  def call
    loaded_people.flat_map { |person| sync_person(person) }
  end

  private

  attr_reader :people, :evidence_scope

  def loaded_people
    @loaded_people ||= people.includes(person_medications: :medication, schedules: :medication).to_a
  end

  def evidence_records
    @evidence_records ||= evidence_scope.order(:id).to_a
  end

  def sync_person(person)
    assigned_medications(person).combination(2).flat_map do |first_medication, second_medication|
      matching_evidence(first_medication, second_medication).map do |evidence|
        sync_prompt(person, first_medication, second_medication, evidence)
      end
    end
  end

  def assigned_medications(person)
    person_medications = person.person_medications.select(&:active?).map(&:medication)
    scheduled_medications = person.schedules.select(&:active?).map(&:medication)
    (person_medications + scheduled_medications).uniq(&:id).sort_by(&:id)
  end

  def matching_evidence(first_medication, second_medication)
    evidence_corpus.matches_for(first_medication.display_name, second_medication.display_name)
  end

  def sync_prompt(person, first_medication, second_medication, evidence)
    primary_medication, interacting_medication = ordered_medications(first_medication, second_medication, evidence)
    prompt = MedicationReviewPrompt.find_or_initialize_by(
      household: person.household,
      person: person,
      primary_medication: primary_medication,
      interacting_medication: interacting_medication,
      evidence_record: evidence
    )
    return prompt if prompt.persisted?

    prompt.assign_attributes(snapshot_attributes(primary_medication, interacting_medication, evidence))
    prompt.save!
    prompt
  end

  def ordered_medications(first_medication, second_medication, evidence)
    return [first_medication, second_medication] if evidence_corpus.owner?(evidence, first_medication.display_name)

    [second_medication, first_medication]
  end

  def evidence_corpus
    @evidence_corpus ||= MedicationReviewEvidenceCorpus.new(evidence_records)
  end

  def snapshot_attributes(primary_medication, interacting_medication, evidence)
    {
      status: default_status(evidence),
      risk_level: evidence.risk_level,
      match_confidence: evidence.match_confidence,
      primary_medication_name: primary_medication.display_name,
      interacting_medication_name: interacting_medication.display_name,
      evidence_source_name: evidence.source_name,
      evidence_source_url: evidence.source_url,
      evidence_source_checked_on: evidence.retrieved_on,
      evidence_text: evidence.evidence_text
    }
  end

  def default_status(evidence)
    return 'hidden_low_signal' if evidence.risk_level == 'low' || evidence.match_confidence == 'low'

    'needs_review'
  end
end
