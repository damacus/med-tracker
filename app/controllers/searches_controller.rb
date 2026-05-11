# frozen_string_literal: true

class SearchesController < ApplicationController
  def show
    respond_to do |format|
      format.html { render Components::GlobalSearch::ShowView.new(query: search_query, results: search_results) }
      format.json { render json: { results: search_results.map(&:as_json) } }
    end
  end

  private

  def search_results
    @search_results ||= GlobalSearchQuery.new(user: current_user, query: search_query).call
  end

  def search_query
    params.expect(:q).to_s
  end
end
