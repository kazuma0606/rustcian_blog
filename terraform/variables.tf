variable "prefix" {
  description = "Short name prefix for all resource names (e.g. 'rustacian')."
  type        = string
  default     = "rustacian"
}

variable "location" {
  description = "Azure region for all resources."
  type        = string
  default     = "japaneast"
}

variable "environment" {
  description = "Deployment environment tag ('dev' | 'prod')."
  type        = string
  default     = "dev"
}

variable "app_service_sku" {
  description = "App Service Plan SKU (e.g. 'B1' for dev, 'P1v3' for prod)."
  type        = string
  default     = "B1"
}

variable "container_image" {
  description = "Full Docker image reference for the backend container (e.g. 'ghcr.io/user/blog:latest')."
  type        = string
}

variable "container_port" {
  description = "Port the backend container listens on."
  type        = number
  default     = 8080
}

variable "base_url" {
  description = "Public base URL for sitemap and RSS generation (e.g. 'https://blog.example.com')."
  type        = string
}

variable "admin_auth_mode" {
  description = "Admin authentication mode ('disabled' | 'local-dev' | 'entra')."
  type        = string
  default     = "entra"
}

variable "entra_tenant_id" {
  description = "Azure AD / Entra ID tenant ID for admin authentication."
  type        = string
  default     = ""
}

variable "entra_client_id" {
  description = "Azure AD / Entra ID app registration client ID."
  type        = string
  default     = ""
}

variable "entra_admin_group_id" {
  description = "Entra ID group object ID whose members are granted admin access."
  type        = string
  default     = ""
}

variable "openai_model_capacity" {
  description = "Token-per-minute capacity (thousands) for the OpenAI deployment."
  type        = number
  default     = 10
}
