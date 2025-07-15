require "brut/back_end/seed_data"
class SeedData < Brut::BackEnd::SeedData
  include FactoryBot::Syntax::Methods
  def seed!
    # Create records here.  This method is not expected
    # to be idempotent.  You can (and should) use your 
    # FactoryBot factories here.
    
    10.times do
      create(:guestbook_message, created_at: Date.today - rand(1..100))
    end
  end
end
