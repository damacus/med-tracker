# frozen_string_literal: true

module GlobalSearch
  Result = Data.define(:type, :title, :subtitle, :path, :score) do
    def as_json(*)
      {
        type: type,
        title: title,
        subtitle: subtitle,
        path: path,
        score: score
      }
    end
  end
end
