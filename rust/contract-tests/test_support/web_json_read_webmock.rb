require 'json'
require 'uri'
require 'webmock'

supplement = {
  code: '5021265221301',
  product_name: 'Contract Vitamin',
  generic_name: 'Daily multivitamin food supplement',
  brands: 'Contract Brand',
  quantity: '30 tablets',
  categories_tags_en: ['food supplements', 'vitamins']
}

WebMock.stub_request(:get, %r{\Ahttps://world\.openfoodfacts\.org/api/v2/product/5021265221301\.json})
       .to_return(status: 200, body: { status: 1, product: supplement }.to_json,
                  headers: { 'Content-Type' => 'application/json' })

WebMock.stub_request(:get, %r{\Ahttps://world\.openproductsfacts\.org/api/v2/product/})
       .to_return(status: 200, body: { status: 0 }.to_json,
                  headers: { 'Content-Type' => 'application/json' })

WebMock.stub_request(:get, %r{\Ahttps://world\.openfoodfacts\.org/cgi/search\.pl}).to_return do |request|
  query = URI.decode_www_form(request.uri.query.to_s).to_h['search_terms']
  products = query == 'vitamin contract' ? [supplement] : []
  { status: 200, body: { products: products }.to_json, headers: { 'Content-Type' => 'application/json' } }
end
