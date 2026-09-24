# frozen_string_literal: true

require "bundler/gem_tasks"
require "rb_sys/extensiontask"

GEMSPEC = Gem::Specification.load("pdf_oxide_structured.gemspec")

RbSys::ExtensionTask.new("pdf_oxide_structured", GEMSPEC) do |ext|
  ext.lib_dir = "lib/pdf_oxide_structured"
end

task default: :compile
