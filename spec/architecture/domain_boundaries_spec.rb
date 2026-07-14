# frozen_string_literal: true

require 'rails_helper'
require 'prism'

class MedicationTakeNodeEvaluator
  DIRECT_WRITE_METHODS = %w[
    build create create! create_or_find_by create_or_find_by! delete delete_all destroy destroy! destroy_all
    delete_by decrement_counter destroy_by find_or_create_by find_or_create_by! find_or_initialize_by first_or_create
    first_or_create! import import! increment_counter insert insert! insert_all insert_all! new reset_counters save
    save! touch touch! touch_all update update! update_all update_attribute update_attribute! update_column
    update_columns
    update_counters upsert upsert_all upsert_all!
  ].freeze
  INSTANCE_WRITE_METHODS = %w[
    decrement! delete delete! destroy destroy! increment! save save! toggle! touch touch! update update!
    update_attribute update_attribute! update_column update_columns
  ].freeze
  ASSOCIATION_WRITE_METHODS = %w[<< append clear concat push replace].freeze
  ASSOCIATION_SETTER_METHODS = %w[medication_take_ids= medication_takes=].freeze
  FINDER_METHODS = %w[
    find find_by find_by! find_sole_by first first! last last! sole take take!
  ].freeze
  RELATION_METHODS = %w[
    all distinct eager_load except extending from group having includes joins left_joins left_outer_joins
    limit lock merge offset only order preload reselect reorder reverse_order select unscope where
  ].freeze
  RECORD_COLLECTION_METHODS = %w[records to_a to_ary].freeze
  NON_RELATION_RESULT_METHODS = %w[
    any? async_average async_count async_ids async_maximum async_minimum async_pick async_pluck async_sum average
    calculate count empty? exists? explain ids length loaded? many? maximum minimum none? one? pick pluck size sum
    table_exists? table_name
  ].freeze
  COLLECTION_METHODS = %w[each each_with_index with_index].freeze
  BATCH_METHODS = %w[find_each find_in_batches in_batches].freeze
  BATCH_WRITE_METHODS = %w[delete_all destroy_all touch_all update_all update_counters].freeze
  ENUMERABLE_ELEMENT_TYPES = {
    association: :record,
    batch_enumerator: :relation,
    model_class: :record,
    model_collection: :model_class,
    record_batch_enumerator: :record_collection,
    record_collection: :record,
    record_enumerator: :record,
    relation: :record
  }.freeze
  SQL_WRITE_METHODS = %w[
    delete exec_delete exec_insert exec_query exec_update execute execute_batch insert update
  ].freeze
  SQL_MUTATION_PATTERN = /\A(?:ALTER|CREATE|DELETE|DROP|INSERT|MERGE|REPLACE|TRUNCATE|UPDATE)\b/i
  SQL_CTE_MUTATION_PATTERN = /\b(?:DELETE\s+FROM|INSERT\s+INTO|UPDATE)\b/i
  MEDICATION_TAKES_TABLE_PATTERN = /\bmedication_takes\b/i
  BRANCH_NODE_TYPES = %i[if_node unless_node case_node case_match_node begin_node rescue_node].freeze
  COMPOUND_WRITE_NODE_TYPES = {
    local_variable_or_write_node: :evaluate_local_compound_write,
    local_variable_and_write_node: :evaluate_local_compound_write,
    instance_variable_or_write_node: :evaluate_instance_compound_write,
    instance_variable_and_write_node: :evaluate_instance_compound_write,
    constant_or_write_node: :evaluate_constant_compound_write,
    constant_and_write_node: :evaluate_constant_compound_write
  }.freeze
  NODE_EVALUATORS = {
    Prism::ProgramNode => :evaluate_program,
    Prism::StatementsNode => :evaluate_statements,
    Prism::LocalVariableWriteNode => :evaluate_local_write,
    Prism::InstanceVariableWriteNode => :evaluate_instance_write,
    Prism::ConstantWriteNode => :evaluate_constant_write,
    Prism::CallNode => :evaluate_call,
    Prism::DefNode => :evaluate_def,
    Prism::LocalVariableReadNode => :evaluate_local_read,
    Prism::ItLocalVariableReadNode => :evaluate_it_read,
    Prism::InstanceVariableReadNode => :evaluate_instance_read,
    Prism::ConstantReadNode => :evaluate_constant,
    Prism::ConstantPathNode => :evaluate_constant,
    Prism::ArrayNode => :evaluate_array,
    Prism::StringNode => :evaluate_string,
    Prism::InterpolatedStringNode => :evaluate_interpolated_string,
    Prism::ParenthesesNode => :evaluate_parentheses,
    Prism::ReturnNode => :evaluate_return
  }.freeze

  def call(source)
    result = Prism.parse(source)
    return [] unless result.success?

    initialize_analysis(source)
    collect_method_definitions(result.value)
    converge(result.value)
    @operations.sort_by { |key, _operation| key }.map(&:last)
  end

  def initialize_analysis(source)
    @source = source
    @operations = {}
    @method_definitions = Hash.new { |definitions, name| definitions[name] = [] }
    @active_methods = []
    @active_method_definitions = []
    @constants = {}
    @instance_variables = {}
    @method_parameter_types = Hash.new { |parameters, key| parameters[key] = {} }
    @method_return_types = {}
    @method_explicit_return_types = {}
  end

  def converge(root)
    20.times do
      @changed = false
      evaluate(root, {})
      @method_definitions.each_value do |definitions|
        definitions.each { |definition| evaluate_method(definition) }
      end
      break unless @changed
    end
  end

  def class_names(source)
    result = Prism.parse(source)
    return [] unless result.success?

    MedicationTakeClassNameScanner.new.call(result.value)
  end

  private

  def collect_method_definitions(node)
    return unless node

    @method_definitions[node.name.to_s] << node if node.is_a?(Prism::DefNode)
    node.child_nodes.each { |child| collect_method_definitions(child) }
  end

  def evaluate(node, environment)
    return empty_types unless node

    evaluator = NODE_EVALUATORS[node.class]
    return send(evaluator, node, environment) if evaluator

    evaluate_children(node, environment)
  end

  def evaluate_program(node, environment)
    evaluate(node.statements, environment)
  end

  def evaluate_statements(node, environment)
    node.body.reduce(empty_types) { |_result, statement| evaluate(statement, environment) }
  end

  def evaluate_local_write(node, environment)
    environment[node.name.to_s] = evaluate(node.value, environment)
  end

  def evaluate_instance_write(node, environment)
    assigned_types = evaluate(node.value, environment)
    merge_state(@instance_variables, node.name.to_s, assigned_types)
    assigned_types
  end

  def evaluate_constant_write(node, environment)
    assigned_types = evaluate(node.value, environment)
    merge_state(@constants, node.name.to_s, assigned_types)
    assigned_types
  end

  def evaluate_local_read(node, environment)
    environment.fetch(node.name.to_s, empty_types)
  end

  def evaluate_def(_node, _environment)
    empty_types
  end

  def evaluate_it_read(_node, environment)
    environment.fetch('it', empty_types)
  end

  def evaluate_instance_read(node, _environment)
    @instance_variables.fetch(node.name.to_s, empty_types)
  end

  def evaluate_constant(node, _environment)
    constant_type(constant_path_name(node))
  end

  def evaluate_parentheses(node, environment)
    evaluate(node.body, environment)
  end

  def evaluate_return(node, environment)
    types = evaluate_arguments(node.arguments, environment).reduce(empty_types, &:|)
    definition = @active_method_definitions.last
    merge_state(@method_explicit_return_types, definition_key(definition), types) if definition
    types
  end

  def evaluate_children(node, environment)
    node.child_nodes.reduce(empty_types) { |_result, child| evaluate(child, environment) }
  end

  def empty_types
    Set.new
  end

  def typed(*names)
    Set.new(names)
  end

  def evaluate_array(node, environment)
    element_types = node.elements.reduce(empty_types) { |types, element| types | evaluate(element, environment) }
    collection_types = empty_types
    collection_types << :model_collection if element_types.include?(:model_class)
    collection_types << :record_collection if element_types.include?(:record)
    collection_types
  end

  def constant_type(name)
    return typed(:model_class) if name.split('::').last == 'MedicationTake'
    return typed(:application_class) if %w[ActiveRecord::Base ApplicationRecord].include?(name)

    @constants.fetch(name.split('::').last, empty_types)
  end
end

class MedicationTakeAssignmentEvaluator < MedicationTakeNodeEvaluator
  private

  def evaluate(node, environment)
    return empty_types unless node

    compound_evaluator = COMPOUND_WRITE_NODE_TYPES[node.type]
    return super unless compound_evaluator

    send(compound_evaluator, node, environment)
  end

  def evaluate_local_compound_write(node, environment)
    name = node.name.to_s
    environment[name] = environment.fetch(name, empty_types) | evaluate(node.value, environment)
  end

  def evaluate_instance_compound_write(node, environment)
    assigned_types = evaluate(node.value, environment)
    merge_state(@instance_variables, node.name.to_s, assigned_types)
  end

  def evaluate_constant_compound_write(node, environment)
    assigned_types = evaluate(node.value, environment)
    merge_state(@constants, node.name.to_s, assigned_types)
  end
end

class MedicationTakeControlFlowEvaluator < MedicationTakeAssignmentEvaluator
  private

  def evaluate(node, environment)
    return empty_types unless node
    return super unless BRANCH_NODE_TYPES.include?(node.type)

    evaluate_branch(node, environment)
  end

  def evaluate_branch(node, environment)
    case node.type
    when :if_node, :unless_node then evaluate_if_branch(node, environment)
    when :case_node, :case_match_node then evaluate_case_branch(node, environment)
    when :begin_node then evaluate_begin_branch(node, environment)
    when :rescue_node then evaluate_rescue_branch(node, environment)
    end
  end

  def evaluate_if_branch(node, environment)
    evaluate(node.predicate, environment)
    alternate = node.respond_to?(:subsequent) ? node.subsequent : node.else_clause
    branches = [node.statements, alternate].compact
    evaluate_branches(branches, environment, include_original: alternate.nil?)
  end

  def evaluate_case_branch(node, environment)
    evaluate(node.predicate, environment) if node.respond_to?(:predicate)
    conditions = node.respond_to?(:conditions) ? node.conditions : []
    branches = conditions.filter_map do |condition|
      evaluate_case_condition(condition, environment)
      condition.statements if condition.respond_to?(:statements)
    end
    else_clause = node.respond_to?(:else_clause) ? node.else_clause : nil
    branches << else_clause if else_clause
    evaluate_branches(branches, environment, include_original: else_clause.nil?)
  end

  def evaluate_case_condition(condition, environment)
    condition.child_nodes.each do |child|
      next if condition.respond_to?(:statements) && child.equal?(condition.statements)

      evaluate(child, environment)
    end
  end

  def evaluate_begin_branch(node, environment)
    branches = [node.statements]
    branches << node.rescue_clause if node.respond_to?(:rescue_clause) && node.rescue_clause
    branches << node.else_clause if node.respond_to?(:else_clause) && node.else_clause
    result = evaluate_branches(branches.compact, environment)
    evaluate(node.ensure_clause, environment) if node.respond_to?(:ensure_clause)
    result
  end

  def evaluate_rescue_branch(node, environment)
    branches = [node.statements, node.subsequent].compact
    evaluate_branches(branches, environment, include_original: node.subsequent.nil?)
  end

  def evaluate_branches(branches, environment, include_original: false)
    branch_environments = branches.map do |branch|
      branch_environment = environment.dup
      [evaluate(branch, branch_environment), branch_environment]
    end
    branch_environments << [empty_types, environment.dup] if include_original
    merge_branch_environments(environment, branch_environments.map(&:last))
    branch_environments.reduce(empty_types) { |types, (result, _branch)| types | result }
  end

  def merge_branch_environments(environment, branch_environments)
    branch_environments.flat_map(&:keys).uniq.each do |name|
      environment[name] = branch_environments.reduce(empty_types) do |types, branch|
        types | branch.fetch(name, empty_types)
      end
    end
  end
end

class MedicationTakeSqlValueEvaluator < MedicationTakeControlFlowEvaluator
  private

  def evaluate_string(node, _environment)
    sql_types(node.unescaped)
  end

  def evaluate_interpolated_string(node, environment)
    node.child_nodes.each { |child| evaluate(child, environment) }
    sql_types(interpolated_sql(node, environment))
  end

  def interpolated_sql(node, environment = {})
    node.parts.map { |part| interpolated_sql_part(part, environment) }.join
  end

  def interpolated_sql_part(part, environment)
    return part.unescaped if part.respond_to?(:unescaped)
    return 'medication_takes' if medication_take_table_name_expression?(part, environment)

    ' '
  end

  def medication_take_table_name_expression?(node, environment)
    return false unless node
    return true if medication_take_table_name_call?(node, environment)

    node.child_nodes.any? { |child| medication_take_table_name_expression?(child, environment) }
  end

  def medication_take_table_name_call?(node, environment)
    return false unless node.is_a?(Prism::CallNode) && node.name == :table_name

    evaluate(node.receiver, environment).include?(:model_class)
  end

  def sql_types(sql)
    normalized = strip_sql_prefix(sql)
    return typed(:medication_take_write_sql) if sql_mutation?(normalized) && sql.match?(MEDICATION_TAKES_TABLE_PATTERN)
    return typed(:read_sql) if normalized.match?(/\ASELECT\b/i)

    typed(:other_sql)
  end

  def strip_sql_prefix(sql)
    sql.sub(%r{\A(?:\s|--[^\n]*(?:\n|\z)|/\*.*?\*/)+}m, '')
  end

  def sql_mutation?(sql)
    sql.match?(SQL_MUTATION_PATTERN) || (sql.match?(/\AWITH\b/i) && sql.match?(SQL_CTE_MUTATION_PATTERN))
  end
end

class MedicationTakeScannerEvaluator < MedicationTakeSqlValueEvaluator
  private

  def evaluate_call(node, environment)
    receiver_types = evaluate(node.receiver, environment)
    positional_types, keyword_types = argument_types(node, environment)
    method_name = node.name.to_s

    record_operation(node, method_name, receiver_types, positional_types)
    result_types = call_result_types(receiver_types, method_name)
    result_types |= local_method_return_types(node, method_name)
    evaluate_block(node.block, environment, block_parameter_types(receiver_types, method_name))
    propagate_method_arguments(node, method_name, positional_types, keyword_types)
    result_types
  end

  def argument_types(node, environment)
    positional_types = []
    keyword_types = {}
    call_arguments(node).each do |argument|
      if argument.is_a?(Prism::KeywordHashNode)
        argument.elements.each do |element|
          next evaluate(element, environment) unless element.is_a?(Prism::AssocNode)

          keyword_types[keyword_name(element.key)] = evaluate(element.value, environment)
        end
      else
        positional_types << evaluate(argument, environment)
      end
    end
    [positional_types, keyword_types]
  end

  def call_arguments(node)
    node.arguments&.arguments || []
  end

  def evaluate_arguments(arguments, environment)
    arguments&.arguments&.map { |argument| evaluate(argument, environment) } || []
  end

  def keyword_name(node)
    return node.unescaped.to_s if node.respond_to?(:unescaped)
    return node.value.to_s if node.respond_to?(:value)

    source_slice(node).delete_suffix(':').delete_prefix(':')
  end

  def source_slice(node)
    @source.byteslice(node.location.start_offset...node.location.end_offset)
  end
end

class MedicationTakeSqlWriteEvaluator < MedicationTakeScannerEvaluator
  private

  def sql_write?(node, method_name, receiver_types, positional_types)
    return false unless sql_method_on_connection?(method_name, receiver_types)

    tracked_result = tracked_sql_write(positional_types.first || empty_types)
    return tracked_result unless tracked_result.nil?

    literal_sql_write?(node, receiver_types)
  end

  def sql_method_on_connection?(method_name, receiver_types)
    SQL_WRITE_METHODS.include?(method_name) && receiver_types.intersect?(typed(:connection, :generic_connection))
  end

  def tracked_sql_write(sql_types)
    return true if sql_types.include?(:medication_take_write_sql)
    return false if sql_types.intersect?(typed(:read_sql, :other_sql))

    nil
  end

  def literal_sql_write?(node, receiver_types)
    sql = sql_text(node)
    return receiver_types.include?(:connection) unless sql
    return false unless sql_mutation?(strip_sql_prefix(sql))

    sql.match?(MEDICATION_TAKES_TABLE_PATTERN)
  end

  def sql_text(node)
    argument = call_arguments(node).first
    return unless argument
    return argument.unescaped if argument.respond_to?(:unescaped)
    return unless argument.is_a?(Prism::InterpolatedStringNode)

    interpolated_sql(argument)
  end

  def constant_path_name(node)
    case node
    when Prism::ConstantReadNode
      node.name.to_s
    when Prism::ConstantPathNode
      [constant_path_name(node.parent), node.name.to_s].compact.reject(&:empty?).join('::')
    else
      ''
    end
  end
end

class MedicationTakeBlockEvaluator < MedicationTakeSqlWriteEvaluator
  private

  def block_parameter_types(receiver_types, method_name)
    active_record_types = active_record_block_parameter_types(receiver_types, method_name)
    return active_record_types unless active_record_types.empty?

    conservative_block_parameter_types(receiver_types, method_name)
  end

  def conservative_block_parameter_types(receiver_types, method_name)
    receiver_types.each_with_object(empty_types) do |receiver_type, types|
      yielded_type = enumerable_element_type(receiver_type, method_name)
      types << yielded_type if yielded_type
    end
  end

  def enumerable_element_type(receiver_type, method_name)
    return :record if receiver_type == :batch_enumerator && method_name == 'each_record'

    ENUMERABLE_ELEMENT_TYPES[receiver_type]
  end

  def active_record_block_parameter_types(receiver_types, method_name)
    return empty_types unless active_record_receiver?(receiver_types)

    {
      'in_batches' => typed(:relation),
      'find_in_batches' => typed(:record_collection),
      'each' => typed(:record),
      'find_each' => typed(:record)
    }.fetch(method_name, empty_types)
  end

  def evaluate_block(block, environment, parameter_types)
    return unless block.is_a?(Prism::BlockNode)

    block_environment = environment.dup
    names = parameter_names(block.parameters)
    if names.empty? && parameter_types.any?
      block_environment['it'] = parameter_types
      block_environment['_1'] = parameter_types
    else
      names.each_with_index do |name, index|
        block_environment[name] = index.zero? ? parameter_types : empty_types
      end
    end
    evaluate(block.body, block_environment)
  end

  def parameter_names(node)
    return [] unless node

    names = []
    names << node.name.to_s if node.respond_to?(:name) && node.type.to_s.end_with?('parameter_node')
    names + node.child_nodes.flat_map { |child| parameter_names(child) }
  end
end

class MedicationTakePersistenceScanner < MedicationTakeBlockEvaluator
  private

  def record_operation(node, method_name, receiver_types, positional_types)
    return unless write_operation?(node, method_name, receiver_types, positional_types)

    key = [node.location.start_offset, node.location.end_offset, method_name]
    @operations[key] = method_name
  end

  def write_operation?(node, method_name, receiver_types, positional_types)
    direct_write?(receiver_types, method_name) ||
      instance_write?(receiver_types, method_name) ||
      batch_write?(receiver_types, method_name) ||
      association_write?(receiver_types, method_name) ||
      ASSOCIATION_SETTER_METHODS.include?(method_name) ||
      sql_write?(node, method_name, receiver_types, positional_types) ||
      raw_column_write?(method_name, positional_types)
  end

  def direct_write?(receiver_types, method_name)
    active_record_receiver?(receiver_types) && DIRECT_WRITE_METHODS.include?(method_name)
  end

  def instance_write?(receiver_types, method_name)
    receiver_types.include?(:record) && INSTANCE_WRITE_METHODS.include?(method_name)
  end

  def batch_write?(receiver_types, method_name)
    receiver_types.include?(:batch_enumerator) && BATCH_WRITE_METHODS.include?(method_name)
  end

  def association_write?(receiver_types, method_name)
    receiver_types.include?(:association) && ASSOCIATION_WRITE_METHODS.include?(method_name)
  end

  def raw_column_write?(method_name, positional_types)
    method_name == 'write_column' && positional_types.first&.include?(:record)
  end

  def call_result_types(receiver_types, method_name)
    result_types = basic_call_result_types(receiver_types, method_name)
    return result_types if result_types

    active_record_call_result_types(receiver_types, method_name)
  end

  def basic_call_result_types(receiver_types, method_name)
    association_types = association_result_types(method_name)
    return association_types if association_types

    receiver_call_result_types(receiver_types, method_name)
  end

  def association_result_types(method_name)
    typed(:association) if method_name == 'medication_takes'
  end

  def receiver_call_result_types(receiver_types, method_name)
    return receiver_types if method_name == 'freeze'
    return connection_types(receiver_types) if method_name == 'connection'
    return record_class_types(receiver_types) if method_name == 'class'
    return batch_result_types(receiver_types, method_name) if BATCH_METHODS.include?(method_name)

    transformed_enumerator = transformed_enumerator_types(receiver_types, method_name)
    return transformed_enumerator if transformed_enumerator
    return receiver_types if enumerable_method?(method_name)

    nil
  end

  def transformed_enumerator_types(receiver_types, method_name)
    typed(:record_enumerator) if receiver_types.include?(:batch_enumerator) && method_name == 'each_record'
  end

  def record_class_types(receiver_types)
    typed(:model_class) if receiver_types.include?(:record)
  end

  def enumerable_method?(method_name)
    COLLECTION_METHODS.include?(method_name) || method_name == 'each_record'
  end

  def batch_result_types(receiver_types, method_name)
    return empty_types unless active_record_receiver?(receiver_types)

    {
      'in_batches' => typed(:batch_enumerator),
      'find_in_batches' => typed(:record_batch_enumerator),
      'find_each' => typed(:record_enumerator)
    }.fetch(method_name)
  end

  def active_record_call_result_types(receiver_types, method_name)
    return empty_types unless active_record_receiver?(receiver_types)
    return typed(:relation) if RELATION_METHODS.include?(method_name)
    return typed(:record) if record_result?(method_name)
    return typed(:record_collection) if RECORD_COLLECTION_METHODS.include?(method_name)
    return empty_types if NON_RELATION_RESULT_METHODS.include?(method_name)

    typed(:relation)
  end

  def record_result?(method_name)
    FINDER_METHODS.include?(method_name) || DIRECT_WRITE_METHODS.include?(method_name)
  end

  def active_record_receiver?(receiver_types)
    receiver_types.intersect?(typed(:association, :model_class, :relation))
  end

  def connection_types(receiver_types)
    return typed(:connection) if receiver_types.include?(:model_class)
    return typed(:generic_connection) if receiver_types.include?(:application_class)

    empty_types
  end

  def local_method_return_types(node, method_name)
    return empty_types unless local_method_call?(node)

    @method_return_types.fetch(method_name, empty_types)
  end

  def propagate_method_arguments(node, method_name, positional_types, keyword_types)
    return unless local_method_call?(node)

    @method_definitions.fetch(method_name, []).each do |definition|
      key = definition_key(definition)
      propagate_positional_arguments(definition, key, positional_types)
      propagate_keyword_arguments(definition, key, keyword_types)
    end
  end

  def propagate_positional_arguments(definition, key, argument_types)
    positional_parameter_names(definition).each_with_index do |name, index|
      merge_state(@method_parameter_types[key], name, argument_types.fetch(index, empty_types))
    end
    rest = rest_parameter_name(definition)
    merge_state(@method_parameter_types[key], rest, argument_types.reduce(empty_types, &:|)) if rest
  end

  def propagate_keyword_arguments(definition, key, argument_types)
    keyword_parameter_names(definition).each do |name|
      merge_state(@method_parameter_types[key], name, argument_types.fetch(name, empty_types))
    end
    keyword_rest = keyword_rest_parameter_name(definition)
    return unless keyword_rest

    merge_state(@method_parameter_types[key], keyword_rest, argument_types.values.reduce(empty_types, &:|))
  end

  def local_method_call?(node)
    node.receiver.nil? || node.receiver.is_a?(Prism::SelfNode)
  end

  def evaluate_method(definition)
    method_name = definition.name.to_s
    return if @active_methods.include?(method_name)

    environment = @method_parameter_types.fetch(definition_key(definition), {}).dup
    apply_parameter_defaults(definition, environment)
    @active_methods << method_name
    return_types = inferred_method_return_types(definition, environment)
    merge_state(@method_return_types, method_name, return_types)
  ensure
    @active_methods.delete(method_name)
    @active_method_definitions.delete(definition)
  end

  def inferred_method_return_types(definition, environment)
    @active_method_definitions << definition
    evaluate(definition.body, environment) |
      @method_explicit_return_types.fetch(definition_key(definition), empty_types)
  end

  def apply_parameter_defaults(definition, environment)
    parameters = definition.parameters
    return unless parameters

    parameters.optionals.each { |parameter| apply_parameter_default(parameter, environment) }
    parameters.keywords.each { |parameter| apply_parameter_default(parameter, environment) }
  end

  def apply_parameter_default(parameter, environment)
    return unless parameter.respond_to?(:value) && parameter.value

    name = parameter.name.to_s
    environment[name] = environment.fetch(name, empty_types) | evaluate(parameter.value, environment)
  end

  def positional_parameter_names(definition)
    parameters = definition.parameters
    return [] unless parameters

    (parameters.requireds + parameters.optionals + parameters.posts).map { |parameter| parameter.name.to_s }
  end

  def keyword_parameter_names(definition)
    definition.parameters&.keywords&.map { |parameter| parameter.name.to_s } || []
  end

  def rest_parameter_name(definition)
    rest = definition.parameters&.rest
    rest&.name&.to_s
  end

  def keyword_rest_parameter_name(definition)
    keyword_rest = definition.parameters&.keyword_rest
    keyword_rest&.name&.to_s
  end

  def definition_key(definition)
    [definition.location.start_offset, definition.location.end_offset]
  end

  def merge_state(state, key, new_types)
    return state.fetch(key, empty_types) if key.nil? || new_types.empty?

    old_types = state.fetch(key, empty_types)
    merged_types = old_types | new_types
    return old_types if merged_types == old_types

    state[key] = merged_types
    @changed = true
    merged_types
  end
end

class MedicationTakeClassNameScanner
  def call(root)
    collect(root, [], [])
  end

  private

  def collect(node, namespace, names)
    return names unless node

    if node.is_a?(Prism::ModuleNode)
      collect(node.body, qualified_namespace(namespace, node.constant_path), names)
    elsif node.is_a?(Prism::ClassNode)
      class_namespace = qualified_namespace(namespace, node.constant_path)
      names << class_namespace.join('::')
      collect(node.body, class_namespace, names)
    else
      node.child_nodes.each { |child| collect(child, namespace, names) }
    end
    names
  end

  def qualified_namespace(namespace, constant_path)
    parts = constant_path_name(constant_path).split('::')
    parts.one? ? namespace + parts : parts
  end

  def constant_path_name(node)
    return node.name.to_s if node.is_a?(Prism::ConstantReadNode)
    return '' unless node.is_a?(Prism::ConstantPathNode)

    [constant_path_name(node.parent), node.name.to_s].reject(&:empty?).join('::')
  end
end

MEDICATION_TAKE_BOUNDARY_CONTRACTS = {
  'app/services/medication_administration/record_dose.rb' => {
    class_name: 'MedicationAdministration::RecordDose', operations: { 'create' => 1 }
  },
  'app/services/medication_administration/restore_history.rb' => {
    class_name: 'MedicationAdministration::RestoreHistory',
    operations: { 'find_or_initialize_by' => 1, 'new' => 1, 'save!' => 1 }
  },
  'app/services/medication_administration/historical_data_migration.rb' => {
    class_name: 'MedicationAdministration::HistoricalDataMigration',
    operations: { 'execute' => 1, 'write_column' => 2 }
  }
}.freeze

RSpec.describe 'Medication administration domain boundaries' do
  let(:scanner) { MedicationTakePersistenceScanner.new }

  it 'removes the legacy dose-recording service from production code' do
    offenders = application_sources.filter_map do |path, source|
      path if source.include?('TakeMedicationService')
    end

    expect(offenders).to be_empty
  end

  it 'keeps each medication-take persistence operation inside an exact reviewed boundary', :aggregate_failures do
    expect(persistence_files.keys).to match_array(MEDICATION_TAKE_BOUNDARY_CONTRACTS.keys)

    MEDICATION_TAKE_BOUNDARY_CONTRACTS.each do |path, contract|
      source = application_sources.fetch(path)

      expect(scanner.class_names(source).count(contract.fetch(:class_name))).to eq(1), path
      expect(persistence_files.fetch(path).tally).to eq(contract.fetch(:operations)), path
    end
  end

  it 'detects common model and association creation or bulk-write bypasses', :aggregate_failures do
    {
      'create_or_find_by!' => 'MedicationTake.create_or_find_by!(client_uuid: value)',
      'find_or_create_by!' => 'source.medication_takes.find_or_create_by!(taken_at: value)',
      'first_or_create!' => 'MedicationTake.where(client_uuid: value).first_or_create!',
      'find_or_initialize_by' => 'MedicationTake.find_or_initialize_by(client_uuid: value)',
      'insert_all!' => 'MedicationTake.insert_all!(rows)',
      'upsert_all' => 'source.medication_takes.upsert_all(rows)',
      'update_all' => 'MedicationTake.where(household: household).update_all(taken_at: value)'
    }.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'detects ordinary instance, relation, and counter persistence APIs', :aggregate_failures do
    ordinary_persistence_sources.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'detects association append and replacement bypasses', :aggregate_failures do
    {
      '<<' => 'source.medication_takes << take',
      'concat' => 'source.medication_takes.concat(takes)',
      'replace' => 'source.medication_takes.replace(takes)'
    }.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end

    mapped_mutation = 'source.medication_takes.map { |take| take.destroy! }'
    expect(scanner.call(mapped_mutation)).to include('destroy!')
  end

  it 'detects association aliases, clearing, and assignment bypasses', :aggregate_failures do
    {
      'push' => 'source.medication_takes.push(take)',
      'append' => 'source.medication_takes.append(take)',
      'clear' => 'source.medication_takes.clear',
      'medication_takes=' => 'source.medication_takes = takes',
      'medication_take_ids=' => 'source.medication_take_ids = ids'
    }.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'detects later instance writes and the legacy raw-column mutation shape', :aggregate_failures do
    instance_write = "record = MedicationTake.find_by(id: id)\nrecord.update!(taken_at: value)"
    local_migrator_write = <<~RUBY
      MedicationTake.where(taken_from_location: from).find_each { write_column(it, :taken_from_location_id, into.id) }
      take = MedicationTake.find_by!(id: id)
      connection = take.class.connection
      connection.execute(sql)
    RUBY

    expect(scanner.call(instance_write)).to include('update!')
    expect(scanner.call(local_migrator_write)).to include('write_column', 'execute')
  end

  it 'follows a multiline record alias' do
    source = "take =\n  MedicationTake.find_by!(id: id)\ntake.update!(taken_at: value)"

    expect(scanner.call(source)).to include('update!')
  end

  it 'follows a class alias' do
    source = "model = MedicationTake\nmodel.create!(taken_at: value)"

    expect(scanner.call(source)).to include('create!')
  end

  it 'follows relation and record aliases' do
    source = <<~RUBY
      relation = MedicationTake.where(household: household)
      take = relation.find_by!(id: id)
      take.destroy!
    RUBY

    expect(scanner.call(source)).to include('destroy!')
  end

  it 'follows model and relation batch yields', :aggregate_failures do
    model_find_each = 'MedicationTake.find_each { |take| take.update!(taken_at: value) }'
    relation_each = 'MedicationTake.all.each { |take| take.save! }'
    relation_batches = 'MedicationTake.in_batches { |relation| relation.update_all(taken_at: value) }'
    record_batches = <<~RUBY
      MedicationTake.find_in_batches do |takes|
        takes.each { |take| take.destroy! }
      end
    RUBY

    expect(scanner.call(model_find_each)).to include('update!')
    expect(scanner.call(relation_each)).to include('save!')
    expect(scanner.call(relation_batches)).to include('update_all')
    expect(scanner.call(record_batches)).to include('destroy!')
  end

  it 'follows batch enumerators through enumerable forwarding', :aggregate_failures do
    batch_forwarding_sources.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'follows an instance-variable relation alias' do
    source = <<~RUBY
      class HistoryWriter
        def initialize
          @relation = MedicationTake.where(household: household)
        end

        def call
          @relation.update_all(taken_at: value)
        end
      end
    RUBY

    expect(scanner.call(source)).to include('update_all')
  end

  it 'follows a helper relation return value' do
    source = <<~RUBY
      def medication_takes_scope
        MedicationTake.where(household: household)
      end

      def call
        relation = medication_takes_scope
        relation.create!(taken_at: value)
      end
    RUBY

    expect(scanner.call(source)).to include('create!')
  end

  it 'unions an early explicit helper return with its terminal expression' do
    source = <<~RUBY
      def history_scope
        return MedicationTake.all if enabled

        Person.all
      end

      history_scope.delete_all
    RUBY

    expect(scanner.call(source)).to include('delete_all')
  end

  it 'preserves medication history taint through standard and custom scope chains', :aggregate_failures do
    {
      'unscoped' => 'MedicationTake.unscoped.delete_all',
      'create_with' => 'MedicationTake.create_with(taken_at: value).find_or_create_by!(client_uuid: value)',
      'where.not' => 'MedicationTake.where.not(client_uuid: nil).update_all(taken_at: value)',
      'custom scope' => 'MedicationTake.recently_recorded.where(household: household).destroy_all'
    }.each do |scope, source|
      expect(scanner.call(source)).not_to be_empty, scope
    end
  end

  it 'maps a record through a keyword helper argument' do
    source = <<~RUBY
      def persist(take:)
        take.save!
      end

      take = MedicationTake.find_by!(id: id)
      persist(take: take)
    RUBY

    expect(scanner.call(source)).to include('save!')
  end

  it 'unions medication history types across ordinary control-flow branches', :aggregate_failures do
    branch_sources.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'follows compound aliases and helper defaults', :aggregate_failures do
    compound_and_default_sources.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
  end

  it 'detects raw SQL writes to medication history without flagging reads', :aggregate_failures do
    {
      'execute' => 'ApplicationRecord.connection.execute("UPDATE medication_takes SET taken_at = NOW()")',
      'exec_update' => 'ActiveRecord::Base.connection.exec_update("UPDATE medication_takes SET taken_at = NOW()")',
      'exec_delete' => 'MedicationTake.connection.exec_delete("DELETE FROM medication_takes")',
      'exec_insert' => 'ApplicationRecord.connection.exec_insert("INSERT INTO medication_takes DEFAULT VALUES")'
    }.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end

    expect(scanner.call('ApplicationRecord.connection.execute("SELECT * FROM medication_takes")')).to be_empty
  end

  it 'tracks medication history SQL through values and mutation prefixes', :aggregate_failures do
    sql_write_sources.each do |operation, source|
      expect(scanner.call(source)).to include(operation), source
    end
    expect(scanner.call(select_sql_source)).to be_empty
  end

  it 'resolves an interpolated medication history table name for generic SQL connections', :aggregate_failures do
    mutation = <<~'RUBY'
      ApplicationRecord.connection.execute("DELETE FROM #{MedicationTake.table_name}")
    RUBY
    read = <<~'RUBY'
      ApplicationRecord.connection.execute("SELECT * FROM #{MedicationTake.table_name}")
    RUBY

    expect(scanner.call(mutation)).to include('execute')
    expect(scanner.call(read)).to be_empty
  end

  it 'resolves model aliases in interpolated medication history table names', :aggregate_failures do
    interpolated_table_alias_sources.each do |source|
      expect(scanner.call(source)).to include('execute'), source
    end
    expect(scanner.call(interpolated_alias_select_source)).to be_empty
  end

  it 'counts chained operations independently' do
    operations = scanner.call('MedicationTake.find_or_initialize_by(client_uuid: value).save!')

    expect(operations.tally).to include('find_or_initialize_by' => 1, 'save!' => 1)
  end

  it 'detects writes through a generic model collection that includes medication history' do
    generic_writer = <<~RUBY
      TENANT_MODELS = [Person, MedicationTake].freeze
      TENANT_MODELS.each do |model|
        model.where(household_id: nil).find_each do |record|
          record.household_id = household.id
          record.save!(validate: false)
        end
      end
    RUBY

    expect(scanner.call(generic_writer)).to include('save!')
  end

  def ordinary_persistence_sources
    {
      'update_attribute' => 'MedicationTake.first.update_attribute(:taken_at, value)',
      'update_attribute!' => 'MedicationTake.first.update_attribute!(:taken_at, value)',
      'increment!' => 'MedicationTake.first.increment!(:dose_amount)',
      'decrement!' => 'MedicationTake.first.decrement!(:dose_amount)',
      'toggle!' => 'MedicationTake.first.toggle!(:skip_stock_mutation)',
      'delete_by' => 'MedicationTake.where(household: household).delete_by(id: id)',
      'destroy_by' => 'MedicationTake.destroy_by(household: household)',
      'touch_all' => 'MedicationTake.where(household: household).touch_all',
      'update_counters' => 'MedicationTake.update_counters(id, dose_amount: 1)',
      'increment_counter' => 'MedicationTake.increment_counter(:dose_amount, id)',
      'decrement_counter' => 'MedicationTake.decrement_counter(:dose_amount, id)',
      'reset_counters' => 'MedicationTake.reset_counters(id, :dose_amount)'
    }
  end

  def batch_forwarding_sources
    [
      ['delete_all', 'MedicationTake.in_batches.each { |relation| relation.delete_all }'],
      ['touch_all', 'MedicationTake.in_batches.with_index { |relation, _index| relation.touch_all }'],
      ['save!', 'MedicationTake.in_batches.each_record { |take| take.save! }'],
      ['increment!', each_record_with_index_source],
      ['increment!', 'MedicationTake.find_each.map { |take| take.increment!(:dose_amount) }'],
      ['delete_all', 'MedicationTake.in_batches.map { |relation| relation.delete_all }'],
      ['destroy!', record_batch_source],
      ['update!', 'MedicationTake.find_each.with_index { |take, _index| take.update!(taken_at: value) }'],
      ['delete', 'MedicationTake.find_each.each_with_index { |take, _index| take.delete }']
    ]
  end

  def each_record_with_index_source
    'MedicationTake.in_batches.each_record.with_index { |take, _index| take.increment!(:dose_amount) }'
  end

  def record_batch_source
    <<~RUBY
      MedicationTake.find_in_batches.each do |takes|
        takes.each { |take| take.destroy! }
      end
    RUBY
  end

  def branch_sources
    [
      ['update!', if_branch_source],
      ['destroy!', unless_branch_source],
      ['save!', case_branch_source],
      ['delete_all', rescue_branch_source]
    ]
  end

  def if_branch_source
    <<~RUBY
      take = if enabled
               MedicationTake.first
             else
               Person.first
             end
      take.update!(taken_at: value)
    RUBY
  end

  def unless_branch_source
    <<~RUBY
      take = unless disabled
               MedicationTake.first
             else
               Person.first
             end
      take.destroy!
    RUBY
  end

  def case_branch_source
    <<~RUBY
      take = case source_type
             when 'history' then MedicationTake.first
             else Person.first
             end
      take.save!
    RUBY
  end

  def rescue_branch_source
    <<~RUBY
      def history_scope
        MedicationTake.where(household: household)
      rescue StandardError
        Person.all
      end
      history_scope.delete_all
    RUBY
  end

  def compound_and_default_sources
    [
      ['update_all', instance_compound_source],
      ['save!', local_compound_source],
      ['destroy!', positional_default_source],
      ['update!', keyword_default_source]
    ]
  end

  def instance_compound_source
    <<~RUBY
      @history_scope ||= MedicationTake.where(household: household)
      @history_scope.update_all(taken_at: value)
    RUBY
  end

  def local_compound_source
    <<~RUBY
      take = Person.first
      take &&= MedicationTake.first
      take.save!
    RUBY
  end

  def positional_default_source
    <<~RUBY
      def persist(take = MedicationTake.first)
        take.destroy!
      end
      persist
    RUBY
  end

  def keyword_default_source
    <<~RUBY
      def persist(take: MedicationTake.first)
        take.update!(taken_at: value)
      end
      persist
    RUBY
  end

  def sql_write_sources
    [
      ['execute', local_sql_source],
      ['exec_delete', instance_sql_source],
      ['exec_insert', sql_helper_argument_source],
      ['exec_update', sql_helper_return_source],
      ['execute', interpolated_sql_source],
      ['execute', commented_sql_source],
      ['execute', cte_sql_source]
    ]
  end

  def local_sql_source
    <<~RUBY
      sql = 'UPDATE medication_takes SET taken_at = NOW()'
      ApplicationRecord.connection.execute(sql)
    RUBY
  end

  def instance_sql_source
    <<~RUBY
      @sql = 'DELETE FROM medication_takes'
      ActiveRecord::Base.connection.exec_delete(@sql)
    RUBY
  end

  def sql_helper_argument_source
    <<~RUBY
      def execute_history(sql:)
        ApplicationRecord.connection.exec_insert(sql)
      end
      execute_history(sql: 'INSERT INTO medication_takes DEFAULT VALUES')
    RUBY
  end

  def sql_helper_return_source
    <<~RUBY
      def history_sql
        'UPDATE medication_takes SET taken_at = NOW()'
      end
      ApplicationRecord.connection.exec_update(history_sql)
    RUBY
  end

  def interpolated_sql_source
    <<~'RUBY'
      ApplicationRecord.connection.execute("UPDATE medication_takes SET taken_at = '#{value}'")
    RUBY
  end

  def commented_sql_source
    'ApplicationRecord.connection.execute("/* history repair */ DELETE FROM medication_takes")'
  end

  def cte_sql_source
    <<~RUBY
      ApplicationRecord.connection.execute(
        'WITH stale AS (SELECT id FROM medication_takes) DELETE FROM medication_takes WHERE id IN (SELECT id FROM stale)'
      )
    RUBY
  end

  def select_sql_source
    <<~RUBY
      sql = 'SELECT * FROM medication_takes'
      ApplicationRecord.connection.execute(sql)
    RUBY
  end

  def interpolated_table_alias_sources
    [
      <<~'RUBY',
        model = MedicationTake
        ApplicationRecord.connection.execute("DELETE FROM #{model.table_name}")
      RUBY
      <<~'RUBY',
        @model = MedicationTake
        ApplicationRecord.connection.execute("DELETE FROM #{@model.table_name}")
      RUBY
      <<~'RUBY'
        HISTORY_MODEL = MedicationTake
        ApplicationRecord.connection.execute("DELETE FROM #{HISTORY_MODEL.table_name}")
      RUBY
    ]
  end

  def interpolated_alias_select_source
    <<~'RUBY'
      model = MedicationTake
      ApplicationRecord.connection.execute("SELECT * FROM #{model.table_name}")
    RUBY
  end

  def application_sources
    Rails.root.glob('app/**/*.rb').to_h do |path|
      [path.relative_path_from(Rails.root).to_s, path.read]
    end
  end

  def persistence_files
    application_sources.each_with_object({}) do |(path, source), files|
      operations = scanner.call(source)
      files[path] = operations if operations.any?
    end
  end
end
