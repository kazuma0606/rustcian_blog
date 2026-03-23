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

variable "container_cpu" {
  description = "vCPU allocation for the Container App (e.g. 0.25 for dev, 1.0 for prod)."
  type        = number
  default     = 0.5
}

variable "container_memory" {
  description = "Memory allocation for the Container App (e.g. '1Gi' for dev, '2Gi' for prod)."
  type        = string
  default     = "1Gi"
}

variable "acs_sender_address" {
  description = "ACS Email sender address (e.g. DoNotReply@<verified-acs-domain>)."
  type        = string
  default     = ""
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

variable "entra_redirect_uri" {
  description = "OAuth2 redirect URI registered in the Entra ID app registration (e.g. https://rustacian-blog.com/admin/callback)."
  type        = string
  default     = ""
}

variable "slack_webhook_url" {
  description = "Slack incoming webhook URL for monitor alerts. Leave empty to disable."
  type        = string
  default     = ""
  sensitive   = true
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for rustacian-blog.com (used to purge cache after publish)."
  type        = string
  default     = ""
}

variable "cloudflare_api_token" {
  description = "Cloudflare API token with Cache Purge permission."
  type        = string
  default     = ""
  sensitive   = true
}

variable "openai_model_capacity" {
  description = "Token-per-minute capacity (thousands) for the OpenAI deployment."
  type        = number
  default     = 10
}

variable "acr_sku" {
  description = "Azure Container Registry SKU ('Basic' for dev, 'Standard' for prod)."
  type        = string
  default     = "Basic"
}

variable "github_actions_principal_id" {
  description = "Object ID of the GitHub Actions OIDC service principal (AcrPush role). Run: az ad sp show --id <client-id> --query id -o tsv"
  type        = string
  default     = ""
}

variable "container_image_dev" {
  description = "Full Docker image reference for the dev container (e.g. 'rustacianprodacr.azurecr.io/rustacian-blog:dev-latest'). Set after first deploy."
  type        = string
  default     = ""
}

variable "base_url_dev" {
  description = "Public base URL for the dev environment (Container App FQDN or custom domain). Set after first deploy."
  type        = string
  default     = ""
}
