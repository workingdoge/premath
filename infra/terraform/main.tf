terraform {
  required_version = ">= 1.6.0"
}

variable "cheese_profile" {
  description = "Canonical Squeak runtime profile (Cheese profile). Empty means fallback to runner_profile."
  type        = string
  default     = ""
}

variable "cheese_relpath_override" {
  description = "Canonical override for runner path, relative to this module directory. Empty means use cheese_profile."
  type        = string
  default     = ""
}

variable "runner_profile" {
  description = "DEPRECATED alias for cheese_profile. Named runner profile."
  type        = string
  default     = "local"
}

variable "runner_relpath_override" {
  description = "DEPRECATED alias for cheese_relpath_override."
  type        = string
  default     = ""
}

locals {
  resolved_cheese_profile = var.cheese_profile != "" ? var.cheese_profile : var.runner_profile
  resolved_relpath_override = var.cheese_relpath_override != "" ? var.cheese_relpath_override : var.runner_relpath_override

  cheese_profiles = {
    local                  = "../../tools/ci/executors/local_runner.sh"
    darwin_microvm_vfkit   = "../../tools/ci/executors/darwin_microvm_vfkit_runner.sh"
  }

  cheese_relpath = local.resolved_relpath_override != "" ? local.resolved_relpath_override : lookup(
    local.cheese_profiles,
    local.resolved_cheese_profile,
    local.cheese_profiles["local"],
  )
}

output "premath_cheese_runner" {
  description = "Absolute path to an executable Cheese runner script."
  value       = abspath("${path.module}/${local.cheese_relpath}")
}

output "premath_cheese_profile" {
  description = "Resolved canonical Cheese profile."
  value       = local.resolved_cheese_profile
}

output "premath_executor_runner" {
  description = "DEPRECATED alias for premath_cheese_runner."
  value       = abspath("${path.module}/${local.cheese_relpath}")
}

output "premath_runner_profile" {
  description = "DEPRECATED alias for premath_cheese_profile."
  value       = local.resolved_cheese_profile
}
