require 'rails_helper'

RSpec.describe PdfTextExtractor do
  it 'decodes a glyph mapped to several UTF-16 code units' do
    stream = "2 beginbfchar\n<0001> <00660066>\n<0002> <D83DDE00>\nendbfchar"

    expect(character_map(stream)).to eq(1 => 'ff', 2 => '😀')
  end

  it 'decodes multi-character glyphs in an explicit range mapping' do
    stream = "1 beginbfrange\n<0001> <0002> [<00660069> <00660066006C>]\nendbfrange"

    expect(character_map(stream)).to eq(1 => 'fi', 2 => 'ffl')
  end
end
