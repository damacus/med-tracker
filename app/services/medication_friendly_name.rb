# frozen_string_literal: true

class MedicationFriendlyName
  STOP_WORDS = %w[
    capsule capsules cream drops eye gel inhaler injection liquid ointment oral pad pads powder sachet sachets spray
    sprays suspension syrup tablet tablets
  ].freeze

  def self.derive(name:, code:)
    new(name: name, code: code).derive
  end

  def initialize(name:, code:)
    @name = name
    @code = code
  end

  def derive
    return nil if code.blank? || name.blank?

    friendly = friendly_words.join(' ').delete_suffix(',').strip
    return nil if friendly.blank? || friendly == name

    friendly
  end

  private

  attr_reader :name, :code

  def friendly_words
    cleaned_name.split.take_while { |word| friendly_word?(word) }
  end

  def cleaned_name
    name.to_s.gsub(/\s*\([^)]*\)/, '').squish
  end

  def friendly_word?(word)
    normalized = word.downcase.delete('.,')
    return false if STOP_WORDS.include?(normalized)
    return false if normalized.match?(/\A\d+(?:[.,]\d+)?(?:mg|mcg|g|ml|iu|%)\z/)
    return false if normalized.match?(/\A\d+(?:[.,]\d+)?\z/)

    true
  end
end
