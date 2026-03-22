variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  type = string
}

variable "tenant_id" {
  description = "Azure AD tenant ID."
  type        = string
}

variable "admin_object_id" {
  description = "Object ID of the Terraform principal that can manage secrets."
  type        = string
}

variable "app_insights_connection_string" {
  description = "Application Insights connection string to store as a secret."
  type        = string
  sensitive   = true
}
