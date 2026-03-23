variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  description = "Prefix for resource names. Hyphens are stripped for the ACR name."
  type        = string
}

variable "sku" {
  description = "ACR SKU: 'Basic' for dev, 'Standard' for prod."
  type        = string
  default     = "Basic"
}

variable "container_app_principal_id" {
  description = "Object ID of the Container App's managed identity (granted AcrPull)."
  type        = string
}

variable "github_actions_principal_id" {
  description = "Object ID of the GitHub Actions OIDC service principal (granted AcrPush)."
  type        = string
}
