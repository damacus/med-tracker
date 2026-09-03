# typed: true

module I18n
  sig { params(key: T.any(String, Symbol), options: T.untyped).returns(T.untyped) }
  def self.t(key, **options); end
end
