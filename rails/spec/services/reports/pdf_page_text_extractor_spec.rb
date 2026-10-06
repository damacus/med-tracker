# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PdfPageTextExtractor do
  describe '#pdf_page_references' do
    it 'follows the document page tree rather than indirect-object serialization order' do
      objects = pdf_objects(<<~PDF)
        1 0 obj
        << /Type /Page /Parent 4 0 R >>
        endobj
        2 0 obj
        << /Type /Page /Parent 4 0 R >>
        endobj
        3 0 obj
        << /Type /Catalog /Pages 4 0 R >>
        endobj
        4 0 obj
        << /Type /Pages /Kids [2 0 R 1 0 R] /Count 2 >>
        endobj
      PDF

      expect(pdf_page_references(objects)).to eq(%w[2 1])
    end
  end
end
