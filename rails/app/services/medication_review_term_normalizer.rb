# frozen_string_literal: true

class MedicationReviewTermNormalizer
  DOSAGE_WORDS = %w[
    capsule capsules cream drops gel inhaler injection liquid ointment pad pads powder sachet sachets spray sprays
    suspension syrup tablet tablets
  ].freeze
  STRENGTH_PATTERN = /\A\d+(?:\.\d+)?(?:mg|mcg|g|ml|iu|%)?\z/

  def self.label(value)
    value.to_s.gsub(/\[[^\]]+\]/, ' ').downcase.gsub(/[^[:alnum:]]+/, ' ').squish
  end

  def self.medication(value)
    label(value).split.take_while { |word| DOSAGE_WORDS.exclude?(word) && !word.match?(STRENGTH_PATTERN) }.join(' ')
  end
end
